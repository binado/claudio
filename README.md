# claudio

**claudio** is a lightweight Rust CLI tool that lets you seamlessly switch between different Claude Code configurations using workflow-focused presets.

## Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/binado/claudio.git
cd claudio

# Build and install
cargo install --path .
```

### From Cargo (once published)

```bash
cargo install claudio
```

## Usage

Claudio uses **presets** - complete configuration recipes for running Claude Code. Instead of manually setting environment variables or passing CLI flags every time, create reusable presets for different workflows:

- `minimax-fast` - Minimax provider with MCPs disabled for speed
- `openrouter` - OpenRouter API for accessing multiple models
- `local-dev` - Local development endpoint for testing
- `code-review` - Custom settings optimized for code reviews

**Quick Tip:** You can run presets directly without the `run` keyword:
```bash
claudio my-preset        # Shorthand
claudio run my-preset    # Explicit (equivalent)
```

### Quick Start

Initialize claudio in your project or user directory:

```bash
# Initialize in project directory (./.claudio/)
claudio init --scope project

# Initialize in user directory (~/.claudio/)
claudio init --scope user
```

Add your first preset:

```bash
# Create a new preset (opens in your default editor)
claudio preset add minimax-fast --scope user
```

Edit the preset file:

```json
{
  "name": "minimax-fast",
  "description": "Minimax with MCPs disabled for speed",
  "env": {
    "ANTHROPIC_BASE_URL": "https://api.minimax.io/anthropic",
    "ANTHROPIC_AUTH_TOKEN": "${MINIMAX_API_KEY}",
    "ANTHROPIC_MODEL": "MiniMax-M2.1",
    "ANTHROPIC_SMALL_FAST_MODEL": "MiniMax-M2.1",
    "ANTHROPIC_DEFAULT_SONNET_MODEL": "MiniMax-M2.1",
    "ANTHROPIC_DEFAULT_OPUS_MODEL": "MiniMax-M2.1",
    "ANTHROPIC_DEFAULT_HAIKU_MODEL": "MiniMax-M2.1"
  },
  "args": ["--no-mcp"]
}
```

Run Claude Code with your preset:

```bash
# Set your API key
export MINIMAX_API_KEY="your-api-key-here"

# Run with the preset (both forms are equivalent)
claudio minimax-fast
claudio run minimax-fast

# Pass additional arguments to claude
claudio minimax-fast -- --verbose

# See what command would be executed (dry-run)
claudio minimax-fast --dry-run
# Output: ANTHROPIC_BASE_URL=https://api.minimax.io/anthropic ANTHROPIC_AUTH_TOKEN=sk-xxx ... claude --no-mcp
```

### Managing Presets

<details>
<summary><strong>List presets</strong></summary>

```bash
# List all available presets
claudio preset list

# List presets from specific scope
claudio preset list --scope user
claudio preset list --scope project

# Verbose output with full details
claudio preset list --verbose

# Search for a specific preset
claudio preset list minimax-fast

# Custom fields display
claudio preset list --fields name,description,filepath
claudio preset list --fields all
```

**Example output:**
```
Available presets:

User (~/.claudio/presets/):
  minimax-fast           Minimax with MCPs disabled for speed
  openrouter             OpenRouter API for multiple models

Project (./.claudio/presets/):
  code-review            Code review with custom settings

3 presets found
```
</details>

<details>
<summary><strong>Show preset details</strong></summary>

```bash
# Show raw preset JSON
claudio preset show minimax-fast

# Show resolved preset (with variable substitution and inheritance)
claudio preset show minimax-fast --resolved

# Output raw JSON only
claudio preset show minimax-fast --json
```

**Example output:**
```
Configuration: minimax-fast
Location: /Users/you/.claudio/presets/minimax-fast.json

{
  "name": "minimax-fast",
  "description": "Minimax with MCPs disabled for speed",
  "env": {
    "ANTHROPIC_BASE_URL": "https://api.minimax.io/anthropic",
    "ANTHROPIC_AUTH_TOKEN": "${MINIMAX_API_KEY}",
    "ANTHROPIC_MODEL": "MiniMax-M2.1",
    "ANTHROPIC_SMALL_FAST_MODEL": "MiniMax-M2.1",
    "ANTHROPIC_DEFAULT_SONNET_MODEL": "MiniMax-M2.1",
    "ANTHROPIC_DEFAULT_OPUS_MODEL": "MiniMax-M2.1",
    "ANTHROPIC_DEFAULT_HAIKU_MODEL": "MiniMax-M2.1"
  },
  "args": ["--no-mcp"]
}
```
</details>

<details>
<summary><strong>Add new preset</strong></summary>

```bash
# Create a new preset in user directory
claudio preset add my-preset --scope user

# Create a new preset in project directory
claudio preset add project-config --scope project
```

This creates a new preset file with a default template and opens it in your default editor (`$VISUAL`, `$EDITOR`, or `vi`).
</details>

<details>
<summary><strong>Edit existing preset</strong></summary>

```bash
# Edit a preset
claudio preset edit minimax-fast

# Edit preset in specific scope
claudio preset edit minimax-fast --scope user
```

Opens the preset file in your default editor.
</details>

<details>
<summary><strong>View environment variables</strong></summary>

```bash
# Print environment variables that would be set
claudio preset env minimax-fast

# Format for shell export
claudio preset env minimax-fast --export

# Show resolved values (with variable substitution)
claudio preset env minimax-fast --resolved
```

**Shell usage:**
```bash
# Apply preset env vars to current shell
eval "$(claudio preset env minimax-fast --export)"

# Now run claude directly with these vars
claude
```
</details>

### Settings and Scope

Claudio supports two configuration scopes:

1. **User scope** (`~/.claudio/`) - Global presets available across all projects
2. **Project scope** (`./.claudio/`) - Project-specific presets that shadow user presets

#### Settings File

The settings file (`settings.json`) controls default behavior:

```json
{
  "default_preset": "minimax-fast",
  "color": "cyan",
  "no_color": false,
  "require_preset": false,
  "ignore_user_presets": false,
  "dry_run_show_env": true,
  "presets": []
}
```

**Settings fields:**
- `default_preset` - Preset to use when none is specified
- `color` - Default highlight color (cyan, green, yellow, blue, magenta)
- `no_color` - Disable colored output
- `require_preset` - Error if no preset is available instead of running bare claude
- `ignore_user_presets` - Only use project presets (project settings only)
- `dry_run_show_env` - Include environment variables in `--dry-run` output (default: true)
- `presets` - Inline preset definitions (alternative to separate files)

#### Scope Examples

```bash
# Run with default preset (uses setting from settings.json)
claudio

# Require a preset to be available
claudio --require-preset

# Use specific scope
claudio minimax-fast --scope user
claudio project-config --scope project
```

#### Advanced: Preset Features

**Variable Substitution:**
```json
{
  "env": {
    "ANTHROPIC_AUTH_TOKEN": "${ANTHROPIC_API_KEY}",
    "CUSTOM_VAR": "${MY_CUSTOM_VAR}"
  }
}
```

**Preset Inheritance:**
```json
{
  "name": "minimax-fast",
  "extends": "base",
  "env": {
    "ANTHROPIC_BASE_URL": "https://api.minimax.io/anthropic"
  }
}
```

**Default Prompts:**
```json
{
  "name": "quick-commit",
  "prompt": "Review the git changes and create a commit",
  "args": []
}
```

**Claude Code Settings:**
```json
{
  "name": "code-review",
  "settings": {
    "model": "opus",
    "permissionMode": "acceptEdits",
    "allowedTools": ["Bash", "Edit", "Read", "Grep"],
    "maxBudgetUsd": 5
  }
}
```

#### Custom Home Directory

For testing or isolated environments, use `CLAUDIO_HOME_DIR`:

```bash
# Use a custom home directory
export CLAUDIO_HOME_DIR=/tmp/test-claudio
mkdir -p /tmp/test-claudio/.claudio/presets
claudio preset list
```

## Development

### Project Structure

Claudio is organized as a Cargo workspace with two crates:

- **`claudio-core`** (`crates/claudio-core/`) - Core domain logic for presets, settings, and resolution
- **`claudio`** (`crates/claudio/`) - CLI binary and command implementations

This separation keeps the core logic independent from CLI-specific dependencies, making it testable and reusable.

### Setup

After cloning the repository, install git hooks using [prek](https://github.com/j178/prek):

```bash
# Install prek
cargo install --locked prek

# Install git hooks
prek install
```

This sets up git hooks that automatically run:
- `cargo fmt` - Code formatting check (pre-commit)
- `cargo clippy` - Linting (pre-commit)

### Building and Testing

```bash
# Build the workspace
cargo build

# Run tests
cargo test

# Format code
cargo fmt

# Run lints
cargo clippy

# Build release version
cargo build --release
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

MIT License - see [LICENSE](LICENSE) file for details.
