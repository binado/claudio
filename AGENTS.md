# Claudio Design Document

## Architecture Overview

Claudio is a preset-based configuration manager for Claude Code. It shifts from provider-centric to workflow-centric configuration, allowing users to create reusable presets for different use cases.

### Core Concepts

- **Presets** - Complete configuration recipes (env vars, CLI args, settings) stored as JSON files
- **Scopes** - Two configuration levels: project-local (`./.claudio/`) and user-level (`~/.claudio/`)
- **Settings** - Per-scope configuration for defaults, inline presets, and behavior options
- **Variable Substitution** - `${VAR_NAME}` syntax for environment variable interpolation
- **Inheritance** - Presets can extend base presets using the `extends` field

## Preset File Format

```json
{
  "name": "preset-name",
  "description": "Human-readable description",
  "extends": "base-preset-name",
  "prompt": "Default prompt to pass to claude",
  "env": {
    "ENV_VAR": "value or ${SUBSTITUTION}"
  },
  "args": ["--flag", "--option", "value"],
  "settings": {
    "model": "sonnet",
    "permissionMode": "acceptEdits"
  }
}
```

### Field Definitions

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | Yes | Preset identifier (should match filename) |
| `description` | string | No | Human-readable description |
| `extends` | string | No | Base preset to inherit from |
| `prompt` | string | No | Default prompt for Claude Code |
| `env` | object | No | Environment variables to set |
| `args` | array | No | CLI arguments to pass to `claude` |
| `settings` | object | No | Claude Code settings (passed via `--settings`) |

### Inheritance Rules

- **env**: Merged (derived preset overrides base)
- **args**: Appended (base args + derived args)
- **settings**: Deep merged (derived overrides base)
- **prompt**: Not inherited (only from current preset)

## Commands

### `claudio init`

Initialize preset directory structure.

```bash
claudio init --scope [auto|project|user]
```

Creates:
- Preset directory (`presets/`)
- Settings file (`settings.json`)

### `claudio [run] <preset>`

Run Claude Code with a preset. The `run` keyword is optional.

```bash
claudio <preset> [--scope SCOPE] [-- CLAUDE_ARGS...]
claudio run <preset> [--scope SCOPE] [-- CLAUDE_ARGS...]  # Explicit form
```

1. Finds preset file in specified scope
2. Resolves inheritance chain
3. Substitutes environment variables
4. Writes settings to temporary file (if present)
5. Executes `claude` with combined env vars and args

### `claudio preset list`

List available presets.

```bash
claudio preset list [NAME] [--scope SCOPE] [--verbose] [--fields FIELDS]
```

Options:
- `NAME` - Filter by preset name
- `--verbose` - Show detailed information
- `--fields` - Customize displayed fields (name, description, filepath, env, args, extends, prompt)

### `claudio preset show <preset>`

Display preset details.

```bash
claudio preset show <preset> [--resolved] [--json]
```

- `--resolved` - Show resolved configuration (after variable substitution and inheritance)
- `--json` - Output raw JSON only

### `claudio preset add <preset>`

Create a new preset.

```bash
claudio preset add <preset> [--scope SCOPE]
```

Creates preset file with default template and opens in editor.

### `claudio preset edit <preset>`

Edit existing preset.

```bash
claudio preset edit <preset> [--scope SCOPE]
```

Opens preset file in `$VISUAL`, `$EDITOR`, or `vi`.

### `claudio preset env <preset>`

Print environment variables.

```bash
claudio preset env <preset> [--export] [--resolved]
```

- `--export` - Format for shell evaluation
- `--resolved` - Show variable substitution sources

## Settings File

Location: `[scope]/.claudio/settings.json`

```json
{
  "default_preset": "minimax-fast",
  "color": "cyan",
  "no_color": false,
  "require_preset": false,
  "ignore_user_presets": false,
  "presets": []
}
```

### Settings Fields

| Field | Type | Description |
|-------|------|-------------|
| `default_preset` | string | Preset to use when none specified |
| `color` | string | Default highlight color (cyan, green, yellow, blue, magenta) |
| `no_color` | boolean | Disable colored output |
| `require_preset` | boolean | Error if no preset available |
| `ignore_user_presets` | boolean | Only use project presets (project scope only) |
| `presets` | array | Inline preset definitions |

## Scope Resolution

### Read Operations (list, show, env, run)

**Auto scope** (default):
1. Check project directory (`./.claudio/presets/`)
2. Check user directory (`~/.claudio/presets/`)
3. Use first match (project shadows user)

**Project scope**:
- Only search `./.claudio/presets/`

**User scope**:
- Only search `~/.claudio/presets/`

### Write Operations (init, add, edit)

**Auto scope**:
- Prefer project if in a git repository
- Otherwise use user directory

**Project/User scope**:
- Write to specified scope

## Environment Variable: CLAUDIO_HOME_DIR

Override user-level home directory for testing:

```bash
export CLAUDIO_HOME_DIR=/tmp/test-claudio
claudio preset list
```

Requirements:
- Must be absolute path
- Must be existing directory
- Used instead of system home directory

## Variable Substitution

Syntax: `${VAR_NAME}`

Works in:
- `env` object values
- `settings` object values (recursively)

Example:
```json
{
  "env": {
    "ANTHROPIC_AUTH_TOKEN": "${ANTHROPIC_API_KEY}"
  },
  "settings": {
    "systemPrompt": "${CUSTOM_PROMPT}"
  }
}
```

Error if referenced variable doesn't exist.

## Implementation Details

### Module Structure

Claudio is organized as a Cargo workspace with two crates:

**claudio-core** (`crates/claudio-core/src/`):
```
├── lib.rs                # Library exports
├── scope.rs              # Scope enum (Auto, Project, User)
├── preset/
│   ├── types.rs         # Preset data structures
│   ├── loader.rs        # File I/O and discovery
│   ├── resolver.rs      # Inheritance and variable substitution
│   └── store.rs         # Preset store abstraction
└── settings/
    ├── types.rs         # Settings data structure
    └── loader.rs        # Settings file I/O
```

**claudio** (`crates/claudio/src/`):
```
├── cli.rs                # CLI argument parsing
├── main.rs              # Entry point
├── color.rs             # Terminal color utilities
├── util.rs              # Helper functions
└── commands/
    ├── init.rs          # Initialize directories
    ├── run.rs           # Run Claude Code
    ├── add.rs           # Create preset
    ├── edit.rs          # Edit preset
    ├── list.rs          # List presets
    ├── show.rs          # Show preset details
    ├── env.rs           # Print environment
    └── editor.rs        # Editor utilities
```

### Key Dependencies

- `clap` - CLI argument parsing
- `serde` / `serde_json` - JSON serialization
- `regex` - Variable substitution
- `directories` - Cross-platform path resolution
- `anyhow` - Error handling
- `comfy-table` - Table formatting
- `colored` - Terminal colors
- `tempfile` - Temporary settings files

### Validation

Preset validation checks:
- Required fields (`name`)
- Environment variable key format (uppercase, underscores, digits)
- Settings must be JSON object
- No circular inheritance
- No self-referencing `extends`
- Filename matches preset name (warning)

Settings validation checks:
- No inline preset name conflicts with file presets
- All inline presets are valid

## Example Presets

### Minimax (Fast Mode)

```json
{
  "name": "minimax-fast",
  "description": "Minimax with MCPs disabled",
  "env": {
    "ANTHROPIC_BASE_URL": "https://api.minimax.chat/v1",
    "ANTHROPIC_AUTH_TOKEN": "${MINIMAX_API_KEY}",
    "ANTHROPIC_DEFAULT_SONNET_MODEL": "minimax-sonnet"
  },
  "args": ["--no-mcp"]
}
```

### OpenRouter (Multiple Models)

```json
{
  "name": "openrouter",
  "description": "OpenRouter API for multiple models",
  "env": {
    "ANTHROPIC_BASE_URL": "https://openrouter.ai/api/v1",
    "ANTHROPIC_AUTH_TOKEN": "${OPENROUTER_API_KEY}",
    "ANTHROPIC_DEFAULT_SONNET_MODEL": "anthropic/claude-sonnet-4"
  }
}
```

### Code Review (Custom Settings)

```json
{
  "name": "code-review",
  "description": "Code review preset with custom settings",
  "settings": {
    "model": "opus",
    "permissionMode": "acceptEdits",
    "allowedTools": ["Bash", "Edit", "Read", "Grep"],
    "maxBudgetUsd": 5
  },
  "env": {
    "ANTHROPIC_AUTH_TOKEN": "${MINIMAX_API_KEY}"
  },
  "args": ["--verbose"]
}
```

### With Inheritance

**Base:**
```json
{
  "name": "base",
  "env": {
    "ANTHROPIC_DEFAULT_SONNET_MODEL": "claude-sonnet-4-20250514"
  },
  "args": ["--verbose"]
}
```

**Derived:**
```json
{
  "name": "minimax",
  "extends": "base",
  "env": {
    "ANTHROPIC_BASE_URL": "https://api.minimax.chat/v1",
    "ANTHROPIC_AUTH_TOKEN": "${MINIMAX_API_KEY}"
  }
}
```

**Resolved:**
```json
{
  "name": "minimax",
  "env": {
    "ANTHROPIC_BASE_URL": "https://api.minimax.chat/v1",
    "ANTHROPIC_AUTH_TOKEN": "${MINIMAX_API_KEY}",
    "ANTHROPIC_DEFAULT_SONNET_MODEL": "claude-sonnet-4-20250514"
  },
  "args": ["--verbose"]
}
```

## Testing Strategy

### Unit Tests
- Preset validation logic (`preset/types.rs`)
- Variable substitution (`preset/resolver.rs`)
- Inheritance resolution (`preset/resolver.rs`)
- Settings validation (`settings/types.rs`)

### Integration Tests
- Preset loading from disk
- Scope resolution behavior
- Command execution
- Error handling

### Test Utilities
- `tempfile` for isolated test environments
- Mock environment variables
- Temporary preset directories
