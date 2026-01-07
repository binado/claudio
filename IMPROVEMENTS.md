# Improvements, Feature Ideas, and Risk Review

Last reviewed: 2026-01-06

This document summarizes potential improvements, refactors, new feature ideas, and a prioritized list of weaknesses/vulnerabilities discovered during a quick codebase review.

## Executive summary (highest leverage items)

1. **Fix inline preset resolution end-to-end.** `claudio run` supports inline presets via `find_preset_or_inline`, but inheritance resolution currently assumes every preset has a file on disk (see `ResolvedPreset.source_path` and `loader::find_preset(&preset.name)` in the resolver). This can break purely-inline presets.
2. **Sanitize preset names before turning them into paths.** Preset names are directly interpolated into filenames (`preset_path_for_name(dir, name) -> dir.join(format!("{}.json", name))`), which enables path traversal if the name contains `../` or separators.
3. **Make inheritance scope-aware and deterministic.** Inheritance currently loads base presets via an unscoped search (`loader::find_preset`) even when the caller selected `--scope user` or `--scope project`. This can lead to surprising (and potentially unsafe) shadowing behavior.
4. **Revisit variable substitution timing.** Variable substitution occurs per-preset before merge, so a missing env var in a base preset can fail resolution even if the derived preset overrides that key.

## New feature ideas

- **`claudio preset explain <preset> [--resolved]`**
  - Print a provenance view (where each `env` entry / arg / setting came from) across the inheritance chain.
  - Useful for debugging complex presets.

- **`claudio preset validate [<name>] [--scope] [--strict]`**
  - Validate presets on disk (and inline presets) using existing validation (`Preset::validate`).
  - `--strict` could fail on warnings as well as errors.

- **Redaction controls for secrets**
  - Add `--redact` (default) and `--reveal` to `preset show --resolved` and `preset env --resolved/--export`.
  - Make redaction configurable via settings (e.g., redact keys matching `TOKEN|KEY|SECRET|AUTH`).

- **Shell-aware export**
  - Add `preset env --export --shell bash|zsh|fish|powershell`.
  - Escaping rules differ; today the export format is POSIX-ish.

- **Better machine output**
  - Add `--json` output to `preset env` and `run --dry-run --json` for automation.

## Behavior / UX improvements

- **Support inline presets consistently across commands**
  - `run` supports inline presets.
  - `preset show` and `preset env` currently require a file preset (`find_preset_scoped`), so they cannot show inline presets.
  - Recommendation: resolve presets via a shared function that returns either `{inline preset}` or `{path + preset}`.

- **More informative errors for settings substitution**
  - `resolve_settings_value` errors with `Environment variable 'X' not found` without guidance.
  - The env substitution path has a nicer message suggesting `export X=...`.

- **Make `preset list` resilient to invalid JSON files**
  - `discover_presets*` will pick up any `.json` file in the preset directories.
  - Consider validating the structure and reporting “invalid preset” rows rather than silently treating it as a preset candidate.

## Refactor suggestions

- **Introduce a single “preset repository” API**
  - Today there are multiple overlapping functions: discovery, scoped discovery, `find_preset*`, `find_preset_or_inline`.
  - Create a small abstraction (e.g., `PresetStore`) that:
    - Resolves a preset name + scope to either `Inline(Preset)` or `File { path, preset }`.
    - Exposes “read locations” and “write location” decisions in one place.

- **Make the resolver pure(ish)**
  - The resolver currently reaches back into `loader` for disk lookups.
  - Prefer passing a resolver context with:
    - scope
    - search dirs
    - a callback/trait for fetching presets by name
  - This makes resolution deterministic and easier to test.

- **Rework `ResolvedPreset.source_path`**
  - Inline presets don’t have a file path.
  - Options:
    - Change to `Option<PathBuf>`
    - Or introduce an enum: `PresetSource::Inline` / `PresetSource::File(PathBuf)`

## Weaknesses & vulnerabilities (prioritized)

### 1) Path traversal via preset name (high)

**Where:** preset creation and lookup uses `preset_path_for_name(dir, name)` with no validation.

**Impact:**
- `claudio preset add` could write outside the intended preset directory if a user provides a name containing `..` and/or path separators.
- `claudio preset edit` could open arbitrary `*.json` files reachable from the preset directory via traversal.

**Suggested fix:**
- Enforce a strict preset name format at CLI parse time (and/or in `add`/`edit`): e.g. `^[a-z0-9][a-z0-9_-]*$`.
- Explicitly reject any name containing `/`, `\\`, or `..`.

### 2) Inline presets can fail during resolution (high)

**Where:** resolver loads `source_path` by calling `loader::find_preset(&preset.name)`.

**Impact:**
- Inline presets (from settings) with no corresponding file can fail in `claudio run` when resolution reaches the `source_path` assignment.

**Suggested fix:**
- Represent source as optional or enum (see refactor section).
- Avoid disk lookup for `source_path` when the preset originated inline.

### 3) Scope confusion during inheritance resolution (medium–high)

**Where:** base preset lookup uses unscoped `loader::find_preset(base_name)`.

**Impact:**
- `--scope user` / `--scope project` callers can still end up inheriting from the “wrong” scope if both exist.
- In untrusted project contexts, this can increase the risk of pulling project-provided bases unexpectedly.

**Suggested fix:**
- Add a `resolve_inheritance_scoped(preset, scope)` variant and use `find_preset_scoped` for all base lookups.
- In `Scope::Auto`, the rule is fine (project shadows user) but should be applied consistently.

### 4) Variable substitution happens before merge (medium)

**Where:** `resolve_variables(preset)` runs before the inheritance merge.

**Impact:**
- A base preset containing `${MISSING}` can fail resolution even if the derived preset overrides the same env key with a concrete value.

**Suggested fix:**
- Merge env maps first (respecting the override rules), then run substitution on the final map.
- Alternatively, only substitute (and validate) keys that survive the merge.

### 5) Secret leakage in output (medium)

**Where:** `preset show --resolved` and `preset env --resolved` print resolved environment variables and settings in plaintext.

**Impact:**
- Easy accidental leakage to terminal scrollback, logs, CI artifacts.

**Suggested fix:**
- Add redaction by default; provide an explicit `--reveal`.
- Optionally redact values when `--export` is used, unless `--reveal`.

### 6) Export safety and shell nuances (low–medium)

**Where:** `preset env --export` emits `export KEY="..."`.

**Notes:**
- The code already escapes `\`, `"`, `$`, and backticks, which is a good baseline.
- However, shell behavior still varies (newlines, control characters, different shells).

**Suggested fix:**
- Add tests for newline-containing values.
- Consider `--shell` option (bash/zsh vs fish/powershell) and/or output as JSON for automation.

## Testing recommendations

- Add unit tests for preset name validation (good names, bad names: `../x`, `a/b`, `a\\b`, `a..b`).
- Add a test for running a pure-inline preset (no file on disk) through the resolver.
- Add tests for inheritance scope correctness (base exists in both scopes, ensure `--scope user` chooses user, etc.).
- Add tests for substitution-after-merge behavior (base has missing var, derived overrides).

## Documentation improvements

- Document that `preset env/show` do or do not support inline presets (and align behavior with `run`).
- Add a warning about printing resolved env/settings to stdout (and recommend `--redact`).
- Document `CLAUDIO_HOME_DIR` behavior (already implemented) with a quick testing example.
