# claudio

**claudio** is a lightweight Rust CLI tool that lets you seamlessly switch between different Claude Code model providers on the fly.

## Why claudio?

When working with Claude Code, you might want to:
- Switch between different API providers (Anthropic, OpenRouter, custom endpoints)
- Use different model configurations for different projects
- Test against multiple providers without changing your code
- Keep API keys separate from your workflow

claudio makes this effortless by acting as a smart wrapper around the `claude` binary, injecting the right environment variables based on your provider configuration.

## Installation

### From Source

```bash
# Clone the repository
git clone <repository-url>
cd claudio

# Build and install
cargo install --path .
```

### From Cargo (once published)

```bash
cargo install claudio
```

## Quick Start

### 1. Create a configuration file

Create `~/.claude/providers.json`:

```json
{
  "providers": {
    "anthropic": {
      "api_endpoint": "https://api.anthropic.com",
      "opus_model": "claude-opus-4-20250514",
      "sonnet_model": "claude-sonnet-4-20250514",
      "haiku_model": "claude-haiku-4-20250514"
    },
    "openrouter": {
      "api_endpoint": "https://openrouter.ai/api/v1",
      "opus_model": "anthropic/claude-opus-4",
      "sonnet_model": "anthropic/claude-sonnet-4",
      "haiku_model": "anthropic/claude-haiku-4"
    }
  }
}
```

### 2. Set your API key

```bash
# Set as environment variable
export ANTHROPIC_API_KEY="your-api-key-here"

# Or for OpenRouter
export OPENROUTER_API_KEY="your-openrouter-key"
```

### 3. Run Claude Code with your provider

```bash
# Use Anthropic
claudio anthropic

# Use OpenRouter
claudio openrouter

# Use default (no provider override)
claudio
```

## Usage

### Basic Usage

```bash
# Launch with a specific provider
claudio <provider>

# Use custom config file
claudio <provider> -c /path/to/config.json

# Pass API key directly
claudio anthropic --api-key sk-ant-...

# Forward arguments to Claude Code
claudio anthropic -- --help
claudio openrouter -- --verbose
```

### Command Line Options

```
claudio [PROVIDER] [OPTIONS] [-- CLAUDE_ARGS...]

Arguments:
  [PROVIDER]  Provider name from your config file (optional)

Options:
  -c, --config-file <FILE>  Custom configuration file path
      --api-key <KEY>       API key (overrides environment variable)
  -h, --help                Print help
  -V, --version             Print version

  [CLAUDE_ARGS...]          Arguments to pass to Claude Code
```

### Configuration File

The configuration file uses the following structure:

```json
{
  "providers": {
    "provider_name": {
      "api_endpoint": "https://api.example.com",
      "opus_model": "model-name-opus",
      "sonnet_model": "model-name-sonnet",
      "haiku_model": "model-name-haiku"
    }
  }
}
```

**Default location:** `~/.claude/providers.json`

### Environment Variables

claudio sets the following environment variables when launching Claude Code:

- `ANTHROPIC_BASE_URL` - Provider's API endpoint
- `ANTHROPIC_DEFAULT_OPUS_MODEL` - Opus model name
- `ANTHROPIC_DEFAULT_SONNET_MODEL` - Sonnet model name
- `ANTHROPIC_DEFAULT_HAIKU_MODEL` - Haiku model name
- `ANTHROPIC_AUTH_TOKEN` - API key

### API Key Resolution

API keys are resolved in the following order (first match wins):

1. `--api-key` command line flag
2. `{PROVIDER}_API_KEY` environment variable (e.g., `ANTHROPIC_API_KEY`, `OPENROUTER_API_KEY`)

## Examples

### Example 1: Multiple Providers

```bash
# Work session with Anthropic
export ANTHROPIC_API_KEY="sk-ant-..."
claudio anthropic

# Switch to OpenRouter for testing
export OPENROUTER_API_KEY="sk-or-..."
claudio openrouter
```

### Example 2: Project-Specific Configuration

```bash
# Use project-specific config
claudio mycompany -c ./config/providers.json
```

### Example 3: Pass-Through Mode

```bash
# Run Claude Code normally (no provider override)
claudio

# This is equivalent to just running:
claude
```

### Example 4: Custom Endpoint

```json
{
  "providers": {
    "local": {
      "api_endpoint": "http://localhost:8000",
      "opus_model": "local-opus",
      "sonnet_model": "local-sonnet",
      "haiku_model": "local-haiku"
    }
  }
}
```

```bash
claudio local --api-key test-key
```

## Development

```bash
# Build
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

## Acknowledgments

- Built for use with [Claude Code](https://claude.ai/code)
- Inspired by the need for flexible provider management in AI development workflows
