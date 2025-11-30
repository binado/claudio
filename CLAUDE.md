# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`claudio` is a Rust CLI tool that switches Claude Code model providers on the fly by setting environment variables before launching the `claude` binary. It reads provider configurations from a JSON file and injects the appropriate API endpoints and model names as environment variables.

## Build and Development Commands

```bash
# Build the project
cargo build

# Build in release mode
cargo build --release

# Run the application
cargo run -- [PROVIDER] [OPTIONS]

# Run with a specific provider
cargo run -- anthropic

# Run with custom config file
cargo run -- anthropic -c path/to/providers.json

# Format code
cargo fmt

# Run lints
cargo clippy

# Check for errors without building
cargo check
```

## Architecture

### Core Modules

- **main.rs**: Entry point that orchestrates the flow:
  1. Parses CLI arguments
  2. Loads provider configuration from `~/.claude/providers.json`
  3. Resolves API key (from --api-key flag or `{PROVIDER}_API_KEY` env var)
  4. Builds environment variables for the selected provider
  5. Launches the `claude` binary with injected environment variables

- **cli.rs**: CLI argument parsing using clap with `derive` feature:
  - `provider`: Optional positional argument for provider name
  - `--config-file, -c`: Custom config file path
  - `--api-key`: API key (overrides environment variable)
  - Trailing arguments: Forwarded to the `claude` binary

- **provider.rs**: Provider configuration management:
  - `Provider` struct with model configurations (opus, sonnet, haiku)
  - `get_providers()`: Loads and parses the providers.json file
  - `get_env_vars()`: Converts provider config to environment variables

- **error.rs**: Custom error types:
  - `ConfigFileNotFound`: Config file doesn't exist
  - `ConfigParseError`: JSON parsing failed
  - `ProviderNotFound`: Requested provider not in config
  - `ApiKeyNotFound`: No API key provided via flag or env var
  - `ProcessLaunchFailed`: Failed to launch claude binary

### Configuration File Format

The `providers.json` file (default: `~/.claude/providers.json`) should follow this structure:

```json
{
  "providers": {
    "provider_name": {
      "api_endpoint": "https://api.example.com",
      "opus_model": "model-opus-name",
      "sonnet_model": "model-sonnet-name",
      "haiku_model": "model-haiku-name"
    }
  }
}
```

### Environment Variables Injected

When launching `claude`, the following environment variables are set:

- `ANTHROPIC_BASE_URL`: Provider's API endpoint
- `ANTHROPIC_DEFAULT_OPUS_MODEL`: Opus model name
- `ANTHROPIC_DEFAULT_SONNET_MODEL`: Sonnet model name
- `ANTHROPIC_DEFAULT_HAIKU_MODEL`: Haiku model name
- `ANTHROPIC_AUTH_TOKEN`: API key

### Error Handling Pattern

The codebase uses Rust's `Result` type throughout with a custom `AppError` enum. All library functions return `Result<T>` and use the `?` operator for error propagation. The only place that calls `process::exit()` is in `main.rs` after launching the claude subprocess, which exits with the same status code as the child process.

## Important Implementation Notes

1. **No process::exit in library code**: Only `main()` is allowed to exit the process. All other functions must return `Result<T>` for proper error handling and testability.

2. **PathBuf for file paths**: Use `PathBuf` instead of `String` for all file path operations to handle path manipulation idiomatically.

3. **Minimize cloning**: Use references (`&str`, `&Path`) instead of owned types when possible to avoid unnecessary allocations.

4. **Trailing arguments**: The CLI accepts trailing arguments using clap's `last = true` attribute with `allow_hyphen_values = true`, which are forwarded to the `claude` binary.

5. **API key resolution**: API keys are resolved in this order:
   - `--api-key` command line flag
   - `{PROVIDER}_API_KEY` environment variable (e.g., `ANTHROPIC_API_KEY`)

6. **Default behavior**: When no provider is specified, claudio launches `claude` with no additional environment variables, effectively acting as a pass-through wrapper.
