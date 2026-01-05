# Claudio Design Document: Preset-Based Architecture

## Motivation

### The Problem with Provider-Based Design

The current version of `claudio` is built around the concept of "providers" - predefined API endpoints tied to specific service providers (Anthropic, OpenRouter, etc.). While this works for simple provider switching, it has significant limitations:

1. **Inflexible abstraction** - Users must think in terms of "which provider am I using?" rather than "what am I trying to accomplish?"
2. **Limited configuration scope** - Only supports setting API endpoints and model names, not arbitrary environment variables or CLI flags
3. **Single-purpose configs** - Cannot create specialized presets for different workflows (e.g., "fast coding with MCPs disabled" vs "deep analysis with full context")
4. **Monolithic preset file** - All providers stored in a single `providers.json` file, making it difficult to share, version control, or organize presets

### The Vision: Use-Case Driven Presets

Inspired by tools like `television` (which uses "channels" as workflow recipes), the new design shifts from provider-centric to use-case-centric presets. Instead of asking "which provider?", users ask "which workflow?"

**Examples of use-case driven presets:**
- `minimax-fast` - Uses Minimax provider with MCPs disabled for speed
- `anthropic-deep` - Anthropic with full context and all features enabled
- `local-dev` - Points to local development endpoint for testing
- `project-alpha` - Custom preset for a specific project with specialized settings

### Key Benefits

1. **Workflow-focused** - Presets represent what you're doing, not just which API you're hitting
2. **Maximum flexibility** - Set any environment variable, pass any CLI flag to Claude Code
3. **Composability** - Individual preset files can extend base presets
4. **Discoverability** - `claudio list` shows all available workflows at a glance
5. **Git-friendly** - One file per preset, easy to share and version control
6. **Arbitrary control** - Not limited to Anthropic-specific variables; control the entire Claude Code execution environment

### Design Philosophy

The redesign follows the "television channel" metaphor:
- **Channels (presets)** are complete recipes for running Claude Code
- **Switching channels** means switching entire execution contexts
- **No manual configuration** - Just select the channel and go
- **Exploration encouraged** - Easy to list, inspect, and try different presets

---

## Use Cases

### `claudio run <preset>`

**Purpose:** Run Claude Code with the specified preset.

**Usage:**
```bash
claudio run <preset-name>
claudio run <preset-name> [-- ADDITIONAL_CLAUDE_ARGS...]
```

**Behavior:**
1. Searches for `<preset-name>.json` in the preset directories (see directory hierarchy)
2. Loads the preset file
3. Resolves any variable substitutions (e.g., `${MINIMAX_API_KEY}`)
4. Sets environment variables specified in the preset
5. Constructs the `claude` command with args from preset + any trailing args
6. Executes `claude` with the constructed environment and arguments
7. Exits with the same status code as the `claude` process

**Directory search order:**
1. `./.claudio/presets/` (project-local)
2. `~/.claudio/presets/` (user-level)

**Examples:**

```bash
# Run with minimax-fast preset
claudio run minimax-fast

# Run with preset and pass additional args to claude
claudio run anthropic-deep -- --verbose

# Preset not found
claudio run non-existent
# Error: Preset 'non-existent' not found in:
#   - /Users/binado/project/.claudio/presets/
#   - /Users/binado/.claudio/presets/
```

**Expected output:**
```
Using preset: minimax-fast (/Users/binado/.claudio/presets/minimax-fast.json)
Launching Claude Code...
[Claude Code session begins]
```

**Exit codes:**
- `0` - Claude Code exited successfully
- `1` - Preset not found or invalid
- `N` - Claude Code exited with code N

---

### `claudio list`

**Purpose:** List all available presets with their descriptions.

**Usage:**
```bash
claudio list
claudio list --verbose
```

**Behavior:**
1. Scans preset directories for `.json` files
2. Reads the `name` and `description` fields from each preset
3. Displays presets in a table format, grouped by source directory

**Examples:**

```bash
claudio list
```

**Expected output:**
```
Available presets:

Project (.claudio/presets/):
  project-alpha          Custom preset for Project Alpha

User (~/.claudio/presets/):
  anthropic-deep         Anthropic with full features enabled
  minimax-fast           Minimax with MCPs disabled for speed
  local-dev              Local development endpoint

3 presets found
```

**With --verbose flag:**
```bash
claudio list --verbose
```

**Expected output:**
```
Available presets:

User (~/.claudio/presets/):

  anthropic-deep (/Users/binado/.claudio/presets/anthropic-deep.json)
    Description: Anthropic with full features enabled
    Environment variables: 4
    CLI arguments: 2
    Extends: base

  minimax-fast (/Users/binado/.claudio/presets/minimax-fast.json)
    Description: Minimax with MCPs disabled for speed
    Environment variables: 3
    CLI arguments: 1
    Extends: none

2 presets found
```

---

### `claudio show <preset>`

**Purpose:** Display the full details of a specific preset.

**Usage:**
```bash
claudio show <preset-name>
claudio show <preset-name> --resolved
```

**Behavior:**
1. Locates the preset file
2. Displays the raw JSON content with syntax highlighting (if terminal supports it)
3. With `--resolved` flag, shows the preset after:
   - Variable substitution
   - Inheritance resolution (if `extends` is used)
   - Final environment variables and arguments that would be used

**Examples:**

```bash
claudio show minimax-fast
```

**Expected output:**
```
Configuration: minimax-fast
Location: /Users/binado/.claudio/presets/minimax-fast.json

{
  "name": "minimax-fast",
  "description": "Minimax with MCPs disabled for speed",
  "env": {
    "ANTHROPIC_BASE_URL": "https://api.minimax.chat/v1",
    "ANTHROPIC_AUTH_TOKEN": "${MINIMAX_API_KEY}",
    "ANTHROPIC_DEFAULT_SONNET_MODEL": "minimax-sonnet"
  },
  "args": [
    "--no-mcp"
  ]
}
```

**With --resolved flag:**
```bash
claudio show minimax-fast --resolved
```

**Expected output:**
```
Configuration: minimax-fast (resolved)
Location: /Users/binado/.claudio/presets/minimax-fast.json

Resolved Environment Variables:
  ANTHROPIC_BASE_URL="https://api.minimax.chat/v1"
  ANTHROPIC_AUTH_TOKEN="sk-minimax-12345..." (from $MINIMAX_API_KEY)
  ANTHROPIC_DEFAULT_SONNET_MODEL="minimax-sonnet"

Resolved CLI Arguments:
  --no-mcp

Command that would be executed:
  claude --no-mcp
```

**Example with prompt:**
```bash
claudio show quick-commit
```

**Expected output:**
```
Configuration: quick-commit
Location: /Users/binado/.claudio/presets/quick-commit.json

{
  "name": "quick-commit",
  "description": "Quick commit workflow with default prompt",
  "prompt": "Review the git changes and create a commit with a descriptive message",
  "env": {
    "ANTHROPIC_AUTH_TOKEN": "${ANTHROPIC_API_KEY}"
  },
  "args": []
}
```

**With --resolved flag:**
```bash
claudio show quick-commit --resolved
```

**Expected output:**
```
Configuration: quick-commit (resolved)
Location: /Users/binado/.claudio/presets/quick-commit.json

Resolved Environment Variables:
  ANTHROPIC_AUTH_TOKEN="sk-ant-12345..." (from $ANTHROPIC_API_KEY)

Prompt:
  "Review the git changes and create a commit with a descriptive message"

Command that would be executed:
  claude "Review the git changes and create a commit with a descriptive message"
```

**Error cases:**
```bash
claudio show non-existent
# Error: Preset 'non-existent' not found
```

---

### `claudio edit <preset>`

**Purpose:** Open a preset file in the user's default editor.

**Usage:**
```bash
claudio edit <preset-name>
```

**Behavior:**
1. Locates the preset file (prioritizes user-level over project-level)
2. Determines the editor from environment variables in order:
   - `$VISUAL`
   - `$EDITOR`
   - Falls back to `vi` on Unix, `notepad` on Windows
3. Opens the preset file in the editor
4. Waits for the editor to close
5. Optionally validates the preset after editing

**Examples:**

```bash
# Edit existing preset
claudio edit minimax-fast
# Opens ~/.claudio/presets/minimax-fast.json in $EDITOR

# Edit non-existent preset prompts creation
claudio edit new-preset
# Preset 'new-config' does not exist. Create it? [y/N]: y
# Creating /Users/binado/.claudio/presets/new-config.json...
# [Opens editor with template]
```

**Expected flow:**
1. User runs `claudio edit minimax-fast`
2. Editor opens with the JSON file
3. User makes changes and saves
4. Editor closes
5. claudio validates the JSON
6. Success or error message displayed

**Template for new presets:**
```json
{
  "name": "new-preset",
  "description": "Description of this preset",
  "prompt": "",
  "env": {
    "ANTHROPIC_BASE_URL": "",
    "ANTHROPIC_AUTH_TOKEN": "${API_KEY}"
  },
  "args": []
}
```

**Validation output:**
```
Preset validated successfully.

To use this preset, run:
  claudio run new-preset
```

**Or if invalid:**
```
Error: Invalid JSON in preset file
  Line 5: Unexpected token '}'

Please fix the errors and validate with:
  claudio validate new-preset
```

---

### `claudio which <preset>`

**Purpose:** Show the file path of a preset.

**Usage:**
```bash
claudio which <preset-name>
```

**Behavior:**
1. Searches preset directories for the specified config
2. Returns the absolute path to the preset file
3. Useful for scripting, debugging, or understanding preset precedence

**Examples:**

```bash
claudio which minimax-fast
```

**Expected output:**
```
/Users/binado/.claudio/presets/minimax-fast.json
```

**With multiple matches (project overrides user):**
```bash
claudio which shared-preset
```

**Expected output:**
```
/Users/binado/project/.claudio/presets/shared-config.json

Note: This preset shadows:
  - /Users/binado/.claudio/presets/shared-config.json
```

**Preset not found:**
```bash
claudio which non-existent
```

**Expected output:**
```
Error: Preset 'non-existent' not found in:
  - /Users/binado/project/.claudio/presets/
  - /Users/binado/.claudio/presets/
```

**Exit codes:**
- `0` - Preset found, path printed
- `1` - Preset not found

**Scripting usage:**
```bash
# Get preset path for use in scripts
CONFIG_PATH=$(claudio which minimax-fast)
cat "$CONFIG_PATH"

# Check if preset exists
if claudio which my-preset > /dev/null 2>&1; then
  echo "Preset exists"
fi
```

---

### `claudio env <preset>`

**Purpose:** Print the environment variables that would be set when running a preset, without actually launching Claude Code.

**Usage:**
```bash
claudio env <preset-name>
claudio env <preset-name> --export
claudio env <preset-name> --resolved
```

**Behavior:**
1. Loads the specified preset
2. Resolves variable substitutions
3. Prints environment variables in a readable format
4. With `--export`, formats output for shell evaluation
5. With `--resolved`, shows the source of each value

**Examples:**

```bash
claudio env minimax-fast
```

**Expected output:**
```
Environment variables for preset 'minimax-fast':

ANTHROPIC_BASE_URL=https://api.minimax.chat/v1
ANTHROPIC_AUTH_TOKEN=${MINIMAX_API_KEY}
ANTHROPIC_DEFAULT_SONNET_MODEL=minimax-sonnet
```

**With --export flag:**
```bash
claudio env minimax-fast --export
```

**Expected output:**
```bash
export ANTHROPIC_BASE_URL="https://api.minimax.chat/v1"
export ANTHROPIC_AUTH_TOKEN="sk-minimax-12345..."
export ANTHROPIC_DEFAULT_SONNET_MODEL="minimax-sonnet"
```

**Shell usage:**
```bash
# Apply preset env vars to current shell
eval "$(claudio env minimax-fast --export)"

# Now run claude directly with these vars
claude
```

**With --resolved flag:**
```bash
claudio env minimax-fast --resolved
```

**Expected output:**
```
Environment variables for preset 'minimax-fast' (resolved):

ANTHROPIC_BASE_URL="https://api.minimax.chat/v1"
  Source: preset file

ANTHROPIC_AUTH_TOKEN="sk-minimax-12345..."
  Source: $MINIMAX_API_KEY (from environment)

ANTHROPIC_DEFAULT_SONNET_MODEL="minimax-sonnet"
  Source: preset file
```

**Use cases:**
- **Debugging** - See what environment variables will be set
- **Shell integration** - Export vars to current shell session
- **Scripting** - Use preset env vars in other tools
- **Validation** - Verify variable substitution works correctly

**Exit codes:**
- `0` - Preset found and env vars printed
- `1` - Preset not found or has errors

---

## JSON Format Specification

### Preset File Structure

Each preset is stored as a separate JSON file in one of the preset directories. The file name (without `.json` extension) serves as the preset identifier.

**File location:** `~/.claudio/presets/<preset-name>.json`

### Schema

```json
{
  "name": "string (required)",
  "description": "string (optional)",
  "extends": "string (optional)",
  "prompt": "string (optional)",
  "env": {
    "ENV_VAR_NAME": "value"
  },
  "args": ["array", "of", "strings"]
}
```

### Field Definitions

#### `name` (required, string)
The identifier for this preset. Should match the filename without the `.json` extension.

**Example:**
```json
{
  "name": "minimax-fast"
}
```

**Validation:**
- Must be present
- Should match the filename (warning if mismatch)
- Recommended format: lowercase with hyphens (e.g., `minimax-fast`, `local-dev`)

---

#### `description` (optional, string)
A human-readable description of what this preset is for. Displayed in `claudio list` and `claudio show`.

**Example:**
```json
{
  "description": "Minimax provider with MCPs disabled for faster response times"
}
```

---

#### `extends` (optional, string)
Inherit settings from another configuration. The base preset is loaded first, then the current config's settings override/merge with the base.

**Example:**
```json
{
  "name": "minimax-fast",
  "extends": "base",
  "env": {
    "ANTHROPIC_BASE_URL": "https://api.minimax.chat/v1"
  }
}
```

**Inheritance rules:**
- `env`: Merged (current preset overrides base)
- `args`: Appended (base args + current args)
- `prompt`: Not inherited (only from current preset if specified)
- `name`, `description`: Not inherited (always from current preset)

**Example inheritance:**

`base.json`:
```json
{
  "name": "base",
  "env": {
    "ANTHROPIC_DEFAULT_SONNET_MODEL": "claude-sonnet-4-20250514"
  },
  "args": ["--verbose"]
}
```

`minimax-fast.json`:
```json
{
  "name": "minimax-fast",
  "extends": "base",
  "env": {
    "ANTHROPIC_BASE_URL": "https://api.minimax.chat/v1",
    "ANTHROPIC_DEFAULT_SONNET_MODEL": "minimax-sonnet"
  },
  "args": ["--no-mcp"]
}
```

**Resolved preset:**
```json
{
  "name": "minimax-fast",
  "env": {
    "ANTHROPIC_BASE_URL": "https://api.minimax.chat/v1",
    "ANTHROPIC_DEFAULT_SONNET_MODEL": "minimax-sonnet"
  },
  "args": ["--verbose", "--no-mcp"]
}
```

---

#### `prompt` (optional, string)
A default prompt to pass to the `claude` command as a positional argument. When specified, this prompt is automatically added to the command line.

**Example:**
```json
{
  "name": "quick-commit",
  "description": "Quick commit with a standard prompt",
  "prompt": "Review the changes and create a commit",
  "args": []
}
```

**Behavior:**
- The prompt is passed as a positional argument after all flags and options
- If the user provides additional trailing arguments, they are appended after the prompt
- The prompt can be overridden by providing a different prompt at runtime

**Example usage:**
```bash
# Using preset with prompt
claudio run quick-commit
# Executes: claude "Review the changes and create a commit"

# Overriding the prompt
claudio run quick-commit -- "Fix the bug in authentication"
# Executes: claude "Fix the bug in authentication"

# Adding to the prompt
claudio run quick-commit -- --verbose
# Executes: claude "Review the changes and create a commit" --verbose
```

**Note:** If both `prompt` in preset and trailing arguments are provided, the trailing arguments take precedence for positional arguments, but flags/options are merged.

---

#### `env` (optional, object)
Environment variables to set when running Claude Code. Keys are variable names, values are variable values.

**Supports variable substitution:**
- `${VAR_NAME}` - Substitutes the value of environment variable `VAR_NAME`
- If the referenced environment variable doesn't exist, an error is raised

**Example:**
```json
{
  "env": {
    "ANTHROPIC_BASE_URL": "https://api.minimax.chat/v1",
    "ANTHROPIC_AUTH_TOKEN": "${MINIMAX_API_KEY}",
    "ANTHROPIC_DEFAULT_SONNET_MODEL": "minimax-sonnet",
    "ANTHROPIC_DEFAULT_OPUS_MODEL": "minimax-opus",
    "ANTHROPIC_DEFAULT_HAIKU_MODEL": "minimax-haiku"
  }
}
```

**Common environment variables for Claude Code:**
- `ANTHROPIC_BASE_URL` - API endpoint
- `ANTHROPIC_AUTH_TOKEN` - Authentication token
- `ANTHROPIC_DEFAULT_SONNET_MODEL` - Sonnet model name
- `ANTHROPIC_DEFAULT_OPUS_MODEL` - Opus model name
- `ANTHROPIC_DEFAULT_HAIKU_MODEL` - Haiku model name

**But any environment variable can be set:**
```json
{
  "env": {
    "CUSTOM_VAR": "custom-value",
    "DEBUG": "true",
    "LOG_LEVEL": "info"
  }
}
```

---

#### `args` (optional, array of strings)
Command-line arguments to pass to the `claude` binary. These are appended to the command when launching Claude Code.

**Example:**
```json
{
  "args": [
    "--no-mcp",
    "--model",
    "sonnet"
  ]
}
```

**Behavior:**
- Args from preset are added before any trailing args from the command line
- If the preset has `--no-mcp` and the user runs `claudio run minimax-fast -- --verbose`, the final command is: `claude --no-mcp --verbose`

---

### Complete Example Configurations

#### Example 1: Anthropic (default)
**File:** `~/.claudio/presets/anthropic.json`

```json
{
  "name": "anthropic",
  "description": "Official Anthropic API with all features enabled",
  "env": {
    "ANTHROPIC_BASE_URL": "https://api.anthropic.com",
    "ANTHROPIC_AUTH_TOKEN": "${ANTHROPIC_API_KEY}",
    "ANTHROPIC_DEFAULT_SONNET_MODEL": "claude-sonnet-4-20250514",
    "ANTHROPIC_DEFAULT_OPUS_MODEL": "claude-opus-4-20250514",
    "ANTHROPIC_DEFAULT_HAIKU_MODEL": "claude-haiku-4-20250514"
  },
  "args": []
}
```

---

#### Example 2: Minimax (fast mode)
**File:** `~/.claudio/presets/minimax-fast.json`

```json
{
  "name": "minimax-fast",
  "description": "Minimax provider with MCPs disabled for speed",
  "env": {
    "ANTHROPIC_BASE_URL": "https://api.minimax.chat/v1",
    "ANTHROPIC_AUTH_TOKEN": "${MINIMAX_API_KEY}",
    "ANTHROPIC_DEFAULT_SONNET_MODEL": "minimax-sonnet"
  },
  "args": [
    "--no-mcp"
  ]
}
```

---

#### Example 3: Local development
**File:** `~/.claudio/presets/local-dev.json`

```json
{
  "name": "local-dev",
  "description": "Local development server for testing",
  "env": {
    "ANTHROPIC_BASE_URL": "http://localhost:8000",
    "ANTHROPIC_AUTH_TOKEN": "test-key",
    "ANTHROPIC_DEFAULT_SONNET_MODEL": "local-sonnet",
    "DEBUG": "true"
  },
  "args": [
    "--verbose"
  ]
}
```

---

#### Example 4: With prompt
**File:** `~/.claudio/presets/quick-commit.json`

```json
{
  "name": "quick-commit",
  "description": "Quick commit workflow with default prompt",
  "prompt": "Review the git changes and create a commit with a descriptive message",
  "env": {
    "ANTHROPIC_AUTH_TOKEN": "${ANTHROPIC_API_KEY}"
  },
  "args": []
}
```

**Usage:**
```bash
claudio run quick-commit
# Executes: claude "Review the git changes and create a commit with a descriptive message"
```

---

#### Example 5: With inheritance
**Base config:** `~/.claudio/presets/base.json`

```json
{
  "name": "base",
  "description": "Base preset with common settings",
  "env": {
    "ANTHROPIC_DEFAULT_SONNET_MODEL": "claude-sonnet-4-20250514",
    "ANTHROPIC_DEFAULT_OPUS_MODEL": "claude-opus-4-20250514",
    "ANTHROPIC_DEFAULT_HAIKU_MODEL": "claude-haiku-4-20250514"
  },
  "args": []
}
```

**Derived config:** `~/.claudio/presets/minimax.json`

```json
{
  "name": "minimax",
  "description": "Minimax provider (extends base models)",
  "extends": "base",
  "env": {
    "ANTHROPIC_BASE_URL": "https://api.minimax.chat/v1",
    "ANTHROPIC_AUTH_TOKEN": "${MINIMAX_API_KEY}",
    "ANTHROPIC_DEFAULT_SONNET_MODEL": "minimax-sonnet"
  }
}
```

**Resolved result:**
```json
{
  "name": "minimax",
  "description": "Minimax provider (extends base models)",
  "env": {
    "ANTHROPIC_BASE_URL": "https://api.minimax.chat/v1",
    "ANTHROPIC_AUTH_TOKEN": "${MINIMAX_API_KEY}",
    "ANTHROPIC_DEFAULT_SONNET_MODEL": "minimax-sonnet",
    "ANTHROPIC_DEFAULT_OPUS_MODEL": "claude-opus-4-20250514",
    "ANTHROPIC_DEFAULT_HAIKU_MODEL": "claude-haiku-4-20250514"
  },
  "args": []
}
```

---

### Validation Rules

1. **Valid JSON** - Preset files must be valid JSON
2. **Required fields** - `name` must be present
3. **Type checking**:
   - `name`, `description`, `extends`, `prompt`: strings
   - `env`: object with string values
   - `args`: array of strings
4. **Variable substitution** - Referenced environment variables must exist at runtime
5. **Circular inheritance** - `extends` chains must not create cycles
6. **File name match** - `name` field should match filename (warning only)

---

### Directory Structure

```
~/.claudio/
└── presets/
    ├── anthropic.json
    ├── minimax-fast.json
    ├── local-dev.json
    └── base.json

./.claudio/          # Project-local presets (optional)
└── presets/
    └── project-alpha.json
```

**Preset discovery order:**
1. `./.claudio/presets/` (project-local, highest priority)
2. `~/.claudio/presets/` (user-level)

If a preset exists in both locations, the project-local version takes precedence.

---

## Implementation Plan

### Phase 1: Core Configuration System (Week 1)

#### Step 1.1: Define new types and structures
**Files to create/modify:**
- `src/preset/types.rs` - New preset types
- `src/preset/loader.rs` - Config file loading
- `src/preset/resolver.rs` - Variable substitution and inheritance

**New types:**
```rust
// src/preset/types.rs

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preset {
    pub name: String,
    pub description: Option<String>,
    pub extends: Option<String>,
    pub prompt: Option<String>,
    pub env: Option<HashMap<String, String>>,
    pub args: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct ResolvedPreset {
    pub name: String,
    pub description: Option<String>,
    pub prompt: Option<String>,
    pub env: HashMap<String, String>,
    pub args: Vec<String>,
    pub source_path: PathBuf,
}

impl Preset {
    pub fn validate(&self) -> Result<()> {
        // Validate required fields
        // Check for circular inheritance
        // Validate types
    }
}
```

**Testing:**
- Unit tests for Preset deserialization
- Validation logic tests
- Edge cases (empty presets, missing fields)

---

#### Step 1.2: Implement preset discovery and loading
**Files to create:**
- `src/preset/loader.rs`

**Key functions:**
```rust
// Find all preset files in preset directories
pub fn discover_presets() -> Result<Vec<PathBuf>>

// Load a single preset file
pub fn load_preset(path: &Path) -> Result<Preset>

// Find a preset by name (searches directories)
pub fn find_preset(name: &str) -> Result<PathBuf>

// Get preset directory paths
pub fn get_preset_dirs() -> Vec<PathBuf>
```

**Preset directory resolution:**
1. `./.claudio/presets/` (current directory)
2. `~/.claudio/presets/` (home directory)

**Testing:**
- Test preset discovery with mock file system
- Test directory precedence
- Test error handling (file not found, permission denied)

---

#### Step 1.3: Implement variable substitution and inheritance
**Files to create:**
- `src/preset/resolver.rs`

**Key functions:**
```rust
// Resolve variable substitutions in env values
pub fn resolve_variables(preset: &Preset) -> Result<HashMap<String, String>>

// Resolve inheritance chain
pub fn resolve_inheritance(preset: &Preset) -> Result<ResolvedConfig>

// Merge base preset with derived config
fn merge_presets(base: &Preset, derived: &Preset) -> Preset
```

**Variable substitution logic:**
- Regex pattern: `\$\{([A-Z_][A-Z0-9_]*)\}`
- Replace with `std::env::var()`
- Error if variable not found

**Inheritance logic:**
- Recursively load base configs
- Detect circular dependencies
- Merge env (override), args (append)

**Testing:**
- Variable substitution tests
- Inheritance chain resolution
- Circular dependency detection
- Missing base preset handling

---

### Phase 2: Core Commands (Week 1-2)

#### Step 2.1: Implement `claudio run`
**Files to modify:**
- `src/cli.rs` - Add new CLI structure
- `src/commands/run.rs` - New file

**CLI structure:**
```rust
// src/cli.rs

#[derive(Parser)]
#[command(name = "claudio")]
pub enum Cli {
    Run {
        /// Preset name to use
        preset: String,

        /// Additional arguments to pass to claude
        #[arg(last = true, allow_hyphen_values = true)]
        claude_args: Vec<String>,
    },
    List {
        /// Show verbose output
        #[arg(short, long)]
        verbose: bool,
    },
    Show {
        /// Preset name to show
        preset: String,

        /// Show resolved preset
        #[arg(long)]
        resolved: bool,
    },
    Edit {
        /// Preset name to edit
        preset: String,
    },
    Which {
        /// Preset name to locate
        preset: String,
    },
    Env {
        /// Preset name
        preset: String,

        /// Format output for shell export
        #[arg(long)]
        export: bool,

        /// Show resolved values
        #[arg(long)]
        resolved: bool,
    },
}
```

**Run command implementation:**
```rust
// src/commands/run.rs

pub fn run(config_name: &str, claude_args: Vec<String>) -> Result<i32> {
    // 1. Find preset file
    let config_path = find_config(config_name)?;

    // 2. Load and resolve preset
    let preset = load_config(&config_path)?;
    let resolved = resolve_inheritance(&preset)?;

    // 3. Build environment
    let mut cmd = Command::new("claude");
    for (key, value) in resolved.env {
        cmd.env(key, value);
    }

    // 4. Add arguments
    cmd.args(resolved.args);
    cmd.args(claude_args);

    // 5. Execute
    let status = cmd.status()?;
    Ok(status.code().unwrap_or(1))
}
```

**Testing:**
- Test preset loading and execution
- Test argument passing
- Test environment variable setting
- Test error cases (preset not found, invalid preset)

---

#### Step 2.2: Implement `claudio list`
**Files to create:**
- `src/commands/list.rs`

**Implementation:**
```rust
pub fn list(verbose: bool) -> Result<()> {
    let config_dirs = get_config_dirs();

    for dir in config_dirs {
        let configs = discover_configs_in_dir(&dir)?;

        println!("{}:", dir.display());
        for config_path in configs {
            let preset = load_config(&config_path)?;

            if verbose {
                print_verbose(&preset, &preset_path);
            } else {
                print_summary(&preset);
            }
        }
    }

    Ok(())
}
```

**Testing:**
- Test output formatting
- Test verbose mode
- Test empty preset directories
- Test multiple directories

---

#### Step 2.3: Implement `claudio show`
**Files to create:**
- `src/commands/show.rs`

**Implementation:**
```rust
pub fn show(config_name: &str, resolved: bool) -> Result<()> {
    let config_path = find_config(config_name)?;
    let preset = load_config(&config_path)?;

    if resolved {
        let resolved_preset = resolve_inheritance(&preset)?;
        print_resolved(&resolved_preset);
    } else {
        print_raw(&preset, &preset_path);
    }

    Ok(())
}
```

**Testing:**
- Test raw output
- Test resolved output
- Test JSON formatting
- Test inheritance display

---

#### Step 2.4: Implement `claudio edit`
**Files to create:**
- `src/commands/edit.rs`

**Implementation:**
```rust
pub fn edit(config_name: &str) -> Result<()> {
    let config_path = match find_config(config_name) {
        Ok(path) => path,
        Err(_) => {
            // Prompt to create new preset
            if prompt_create(config_name)? {
                create_config(config_name)?
            } else {
                return Ok(());
            }
        }
    };

    let editor = get_editor()?;
    let status = Command::new(editor)
        .arg(&config_path)
        .status()?;

    if status.success() {
        validate_config(&config_path)?;
    }

    Ok(())
}

fn get_editor() -> Result<String> {
    env::var("VISUAL")
        .or_else(|_| env::var("EDITOR"))
        .or_else(|_| Ok("vi".to_string()))
}
```

**Testing:**
- Test editor detection
- Test creating new configs
- Test validation after edit
- Test error handling

---

#### Step 2.5: Implement `claudio which`
**Files to create:**
- `src/commands/which.rs`

**Implementation:**
```rust
pub fn which(config_name: &str) -> Result<()> {
    let config_path = find_config(config_name)?;
    println!("{}", config_path.display());

    // Check for shadowed configs
    let all_matches = find_all_configs(config_name)?;
    if all_matches.len() > 1 {
        println!("\nNote: This preset shadows:");
        for shadowed in all_matches.iter().skip(1) {
            println!("  - {}", shadowed.display());
        }
    }

    Ok(())
}
```

**Testing:**
- Test finding configs
- Test shadowing detection
- Test error cases

---

#### Step 2.6: Implement `claudio env`
**Files to create:**
- `src/commands/env.rs`

**Implementation:**
```rust
pub fn env(config_name: &str, export: bool, resolved: bool) -> Result<()> {
    let config_path = find_config(config_name)?;
    let preset = load_config(&config_path)?;
    let resolved_preset = resolve_inheritance(&preset)?;

    if export {
        print_export_format(&resolved_preset);
    } else if resolved {
        print_resolved_format(&resolved_preset);
    } else {
        print_simple_format(&resolved_preset);
    }

    Ok(())
}
```

**Testing:**
- Test different output formats
- Test variable resolution
- Test export format for shell

---

### Phase 3: Error Handling and Polish (Week 2)

#### Step 3.1: Enhance error messages
**Files to modify:**
- `src/error.rs`

**New error types:**
```rust
pub enum AppError {
    ConfigNotFound { name: String, searched_paths: Vec<PathBuf> },
    ConfigInvalid { path: PathBuf, reason: String },
    VariableNotFound { var_name: String, config_path: PathBuf },
    CircularInheritance { chain: Vec<String> },
    EditorNotFound,
    ClaudeNotFound,
}
```

**Improve error display:**
- Show helpful context
- Suggest fixes
- Display file paths clearly

---

#### Step 3.2: Add validation command (optional but recommended)
**Files to create:**
- `src/commands/validate.rs`

**Implementation:**
```rust
pub fn validate(config_name: &str) -> Result<()> {
    let config_path = find_config(config_name)?;
    let preset = load_config(&config_path)?;

    preset.validate()?;

    // Try to resolve inheritance
    let _resolved = resolve_inheritance(&preset)?;

    println!("✓ Preset '{}' is valid", config_name);
    Ok(())
}
```

---

#### Step 3.3: Update main.rs
**Files to modify:**
- `src/main.rs`

**Implementation:**
```rust
fn main() {
    let cli = Cli::parse();

    let result = match cli {
        Cli::Run { preset, claude_args } => {
            commands::run::run(&preset, claude_args)
                .map(|code| std::process::exit(code))
        }
        Cli::List { verbose } => {
            commands::list::list(verbose)
        }
        Cli::Show { preset, resolved } => {
            commands::show::show(&preset, resolved)
        }
        Cli::Edit { preset } => {
            commands::edit::edit(&preset)
        }
        Cli::Which { preset } => {
            commands::which::which(&preset)
        }
        Cli::Env { preset, export, resolved } => {
            commands::env::env(&preset, export, resolved)
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
```

---

### Phase 4: Testing and Documentation (Week 2)

#### Step 4.1: Comprehensive testing
**Test files to create:**
- `tests/integration_test.rs`
- `tests/config_loading_test.rs`
- `tests/inheritance_test.rs`
- `tests/cli_test.rs`

**Test coverage:**
- Preset loading from disk
- Variable substitution
- Inheritance resolution
- All commands (run, list, show, edit, which, env)
- Error cases
- Edge cases

**Test utilities:**
- Use `tempfile` for temporary preset directories
- Mock environment variables
- Mock `claude` binary for testing

---

#### Step 4.2: Update documentation
**Files to update:**
- `README.md` - New usage examples and command reference
- `CLAUDE.md` - Updated architecture documentation

**New documentation structure:**
- Quick start guide
- Configuration file format
- Command reference
- Examples and use cases
- Migration guide (optional, since you don't care about backward compatibility)

---

### Phase 5: Optional Enhancements (Future)

#### Future improvements to consider:
1. **JSON Schema validation** - Validate presets against a schema
2. **Preset templates** - Built-in templates for common providers
3. **Shell completion** - Bash/Zsh/Fish completion scripts
4. **Preset validation on edit** - Real-time validation in editor
5. **Import/export** - Share presets easily
6. **YAML support** - If users request it later

---

### Dependencies Required

Add to `Cargo.toml`:

```toml
[dependencies]
clap = { version = "4.5", features = ["derive"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "1.0"
regex = "1.10"              # Custom variable substitution (${VAR_NAME})
directories = "5.0"         # Cross-platform preset directory paths
jsonschema = "0.18"         # JSON Schema validation for preset files
colored = "2.1"             # Terminal color output

# Optional: JSON syntax highlighting for 'claudio show' command
syntect = "5.2"             # Syntax highlighting using Sublime Text definitions

[dev-dependencies]
tempfile = "3.8"
assert_fs = "1.1"
```

**Dependency rationale:**
- **regex** - Custom implementation of `${VAR_NAME}` variable substitution (simpler than shellexpand for our use case)
- **directories** - Battle-tested cross-platform path resolution for preset directories
- **jsonschema** - Robust preset validation (complex problem, use a mature library)
- **colored** - Simple terminal coloring for output (no TUI framework needed)
- **syntect** - Optional JSON syntax highlighting for better UX in `claudio show`

**Custom implementations:**
- **Variable substitution** - Simple regex-based `${VAR}` expansion (~10-15 lines)
- **Editor launching** - Custom `$VISUAL`/`$EDITOR` detection with platform-specific fallbacks (~20 lines)

---

### Timeline Summary

**Week 1:**
- Days 1-2: Core preset system (types, loading, resolution)
- Days 3-5: Core commands (run, list, show)

**Week 2:**
- Days 1-2: Remaining commands (edit, which, env)
- Days 3-4: Error handling and polish
- Day 5: Testing and documentation

**Total: ~10 days of focused development**

---

### Migration Notes

Since backward compatibility is not required:
1. Remove all provider-related code
2. Delete `src/provider.rs`
3. Update CLI structure completely
4. No migration tool needed
5. Clean slate implementation

---

### Success Criteria

The implementation is complete when:
- ✅ All 6 commands work as specified
- ✅ Preset inheritance works correctly
- ✅ Variable substitution works correctly
- ✅ Comprehensive test coverage (>80%)
- ✅ Documentation is updated
- ✅ Error messages are clear and helpful
- ✅ Can successfully run Claude Code with custom configs
