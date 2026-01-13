# List command improvements

Command:

```bash
claudio preset list
```

## Iteration 1

### Potential improvements

1. The `--claude-executable` flag makes no sense in the list command
2. Should we keep `--prompt-max-length` or should we set a max length for the whole table width? `comfy-table` already has a setting for that, could be useful.
3. The highlight color list is currently limited, we could add rgb support with the TrueColor enum value of the `colored` crate. But this could be the subject of a separate issue...
4. The `--scope` flag description could be more clearly written, especially the `auto` flag. In the list command specifically, it makes more sense to replace the `auto` flag with a `both` flag, since listing presets from both scopes is a common use case.
5. Is the verbose flag really useful, since we already provide the --fields all shortcut?

### Additional improvements

6. **Highlight default preset** - If a default preset is configured in settings, mark it distinctly in the list (e.g., with a special symbol like `*` or color indicator in the name column). This provides immediate visual feedback on which preset runs by default.
   - Implementation: Check current preset name against `settings.default_preset`
   - Low complexity, high UX benefit

7. **Group by scope in `both` mode** - When showing presets from both scopes (using the proposed `both` flag), visually separate them with section headers like "Project Presets" and "User Presets". This makes scope precedence crystal clear.
   - Implementation: Sort presets by scope, add section headers before each group
   - Low complexity, high UX benefit

8. **Add `--json` output format** - Add a `--json` flag for machine-readable output, useful for scripting and automation use cases. Skip table formatting and output serialized preset data.
   - Implementation: Use `serde_json::to_string_pretty()` on preset list
   - Low-medium complexity, medium UX benefit

## Iteration 2

### Improvements to implement

6. **Highlight default preset** - If a default preset is configured in settings, mark it distinctly in the list (e.g., with a special symbol like `*` or color indicator in the name column). This provides immediate visual feedback on which preset runs by default.
   - Implementation: Check current preset name against `settings.default_preset`
   - Low complexity, high UX benefit

7. **Group by scope in `both` mode** - When showing presets from both scopes (using the proposed `both` flag), visually separate them with section headers like "Project Presets" and "User Presets". This makes scope precedence crystal clear.
   - Implementation: Sort presets by scope, add section headers before each group
   - Low complexity, high UX benefit

11. **Show which presets are shadowed** - If project scope has a preset with same name as user scope, indicate the user one is shadowed. This helps users understand scope precedence and prevents confusion about why a preset isn't being used.
   - Implementation: Collect presets from both scopes, detect name collisions, add indicator (e.g., "shadowed" label or grayed out)
   - Medium complexity, medium UX benefit
