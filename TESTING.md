# Testing in Rust

This document explains how testing works in Rust and how to apply it to the claudio project.

## Basic Test Structure

Tests in Rust are regular functions annotated with `#[test]`:

```rust
#[test]
fn it_works() {
    assert_eq!(2 + 2, 4);
}
```

## Where Tests Live

Rust supports two main approaches:

### 1. Unit Tests (Same File)
Tests live alongside the code they test, in a `tests` module:

```rust
// src/provider.rs
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        assert_eq!(add(2, 2), 4);
    }
}
```

The `#[cfg(test)]` attribute means this module only compiles when running tests.

### 2. Integration Tests (Separate Directory)
Tests in the `tests/` directory test your crate as an external user would:

```rust
// tests/integration_test.rs
use claudio::provider::get_providers;

#[test]
fn test_provider_loading() {
    // Test using public API only
}
```

## Assertion Macros

Rust provides several assertion macros:

```rust
assert!(true);                          // Assert something is true
assert_eq!(left, right);                // Assert equality
assert_ne!(left, right);                // Assert inequality
panic!("explicit failure");             // Force a test to fail
```

## Testing Error Handling

For functions that return `Result<T, E>`:

```rust
#[test]
fn test_error_case() {
    let result = some_function();
    assert!(result.is_err());

    // Or check specific error type
    match result {
        Err(AppError::ApiKeyNotFound(_)) => {}, // Expected
        _ => panic!("Wrong error type"),
    }
}

// Or use the matches! macro
#[test]
fn test_with_matches() {
    let result = some_function();
    assert!(matches!(result, Err(AppError::ApiKeyNotFound(_))));
}
```

## Running Tests

```bash
# Run all tests
cargo test

# Run tests with output visible (normally hidden for passing tests)
cargo test -- --nocapture

# Run a specific test by name
cargo test test_provider_loading

# Run tests matching a pattern
cargo test provider

# Run with verbose output
cargo test -- --test-threads=1 --nocapture

# Run only integration tests
cargo test --test '*'

# Run only unit tests
cargo test --lib

# Run doc tests
cargo test --doc
```

## Test Organization for claudio

For the `claudio` project, tests should be structured like this:

**Project structure:**
```
claudio/
├── src/
│   ├── lib.rs          # NEW: Exposes modules for testing
│   ├── main.rs
│   ├── cli.rs
│   ├── provider.rs
│   └── error.rs
├── tests/
│   ├── provider_tests.rs
│   └── cli_tests.rs
└── Cargo.toml
```

### Why You Need `src/lib.rs`

Integration tests (in `tests/`) can only use the **public API** of your crate. Since claudio is currently a binary-only crate, there's no public API to test. Adding `lib.rs` exposes your modules:

```rust
// src/lib.rs
pub mod cli;
pub mod error;
pub mod provider;
```

Then update `Cargo.toml`:
```toml
[lib]
name = "claudio"
path = "src/lib.rs"

[[bin]]
name = "claudio"
path = "src/main.rs"
```

Now integration tests can import your modules:
```rust
// tests/provider_tests.rs
use claudio::provider::{Provider, get_providers};
use claudio::error::AppError;
```

## Common Testing Patterns

### 1. Testing with Temporary Files

Use the `tempfile` crate to create temporary files for testing:

```rust
use tempfile::NamedTempFile;
use std::io::Write;

#[test]
fn test_with_temp_file() {
    // Create a temporary file
    let mut temp_file = NamedTempFile::new().unwrap();

    // Write test data
    write!(temp_file, r#"{{"test": "data"}}"#).unwrap();

    // Use the file in your test
    let result = parse_config(temp_file.path());

    assert!(result.is_ok());

    // File is automatically deleted when temp_file goes out of scope
}
```

### 2. Testing with Environment Variables

```rust
use std::env;

#[test]
fn test_env_var() {
    // Set environment variable
    env::set_var("TEST_API_KEY", "secret-key");

    // Run test
    let key = get_api_key("test").unwrap();
    assert_eq!(key, "secret-key");

    // Clean up
    env::remove_var("TEST_API_KEY");
}
```

**Warning:** Environment variables are global, so tests that modify them can interfere with each other when run in parallel. Use `cargo test -- --test-threads=1` if you have issues.

### 3. Testing Error Cases

```rust
#[test]
fn test_file_not_found() {
    let result = get_providers(&PathBuf::from("/nonexistent/path.json"));

    // Check that it's an error
    assert!(result.is_err());

    // Check the specific error type
    match result {
        Err(AppError::ConfigFileNotFound(path)) => {
            assert_eq!(path, PathBuf::from("/nonexistent/path.json"));
        }
        _ => panic!("Expected ConfigFileNotFound error"),
    }
}

// More concise with matches! macro
#[test]
fn test_file_not_found_concise() {
    let result = get_providers(&PathBuf::from("/nonexistent/path.json"));
    assert!(matches!(result, Err(AppError::ConfigFileNotFound(_))));
}
```

### 4. Testing Success Cases

```rust
#[test]
fn test_provider_env_vars() {
    let provider = Provider {
        api_endpoint: "https://api.example.com".to_string(),
        opus_model: "opus-v1".to_string(),
        sonnet_model: "sonnet-v1".to_string(),
        haiku_model: "haiku-v1".to_string(),
    };

    let vars = provider.get_env_vars("test-key-123".to_string());

    // Check the number of variables
    assert_eq!(vars.len(), 5);

    // Check specific values
    assert!(vars.contains(&("ANTHROPIC_BASE_URL".to_string(), "https://api.example.com".to_string())));
    assert!(vars.contains(&("ANTHROPIC_AUTH_TOKEN".to_string(), "test-key-123".to_string())));
}
```

## Advanced Features

### Should Panic Tests

Test that code panics as expected:

```rust
#[test]
#[should_panic(expected = "index out of bounds")]
fn test_panic() {
    let v = vec![1, 2, 3];
    v[99]; // Should panic
}
```

### Ignoring Tests

Skip expensive or flaky tests by default:

```rust
#[test]
#[ignore]
fn expensive_integration_test() {
    // Only runs with: cargo test -- --ignored
}
```

### Test Fixtures with Setup/Teardown

```rust
struct TestContext {
    temp_file: NamedTempFile,
}

impl TestContext {
    fn new() -> Self {
        let mut temp_file = NamedTempFile::new().unwrap();
        write!(temp_file, r#"{{"providers": {}}}"#).unwrap();
        TestContext { temp_file }
    }
}

#[test]
fn test_with_fixture() {
    let ctx = TestContext::new();

    // Test code here
    let result = get_providers(ctx.temp_file.path());

    assert!(result.is_ok());

    // Cleanup happens automatically via Drop trait
}
```

### Documentation Tests

Tests in documentation comments are executable:

```rust
/// Adds two numbers together.
///
/// # Examples
///
/// ```
/// use claudio::add;
///
/// let result = add(2, 2);
/// assert_eq!(result, 4);
/// ```
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
```

Run with `cargo test --doc`.

## Example Tests for claudio

### Provider Tests

```rust
// tests/provider_tests.rs
use claudio::provider::{Provider, get_providers};
use std::path::PathBuf;
use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn test_provider_env_vars() {
    let provider = Provider {
        api_endpoint: "https://api.example.com".to_string(),
        opus_model: "opus-v1".to_string(),
        sonnet_model: "sonnet-v1".to_string(),
        haiku_model: "haiku-v1".to_string(),
    };

    let vars = provider.get_env_vars("test-key-123".to_string());

    assert_eq!(vars.len(), 5);
    assert!(vars.contains(&("ANTHROPIC_BASE_URL".to_string(), "https://api.example.com".to_string())));
    assert!(vars.contains(&("ANTHROPIC_AUTH_TOKEN".to_string(), "test-key-123".to_string())));
}

#[test]
fn test_get_providers_file_not_found() {
    let result = get_providers(&PathBuf::from("/nonexistent/path.json"));
    assert!(result.is_err());
}

#[test]
fn test_get_providers_invalid_json() {
    let mut temp_file = NamedTempFile::new().unwrap();
    write!(temp_file, "{{ invalid json").unwrap();

    let result = get_providers(temp_file.path());
    assert!(result.is_err());
}

#[test]
fn test_get_providers_valid_config() {
    let mut temp_file = NamedTempFile::new().unwrap();
    write!(temp_file, r#"{{
        "providers": {{
            "test": {{
                "api_endpoint": "https://test.com",
                "opus_model": "opus",
                "sonnet_model": "sonnet",
                "haiku_model": "haiku"
            }}
        }}
    }}"#).unwrap();

    let result = get_providers(temp_file.path());
    assert!(result.is_ok());

    let providers = result.unwrap();
    assert_eq!(providers.len(), 1);
    assert!(providers.contains_key("test"));

    let provider = providers.get("test").unwrap();
    assert_eq!(provider.api_endpoint, "https://test.com");
    assert_eq!(provider.opus_model, "opus");
}
```

### CLI Tests

```rust
// tests/cli_tests.rs
use claudio::cli::Cli;
use claudio::error::AppError;
use std::env;

#[test]
fn test_get_api_key_from_cli() {
    let cli = Cli {
        provider: Some("test".to_string()),
        config_file: None,
        api_key: Some("cli-key".to_string()),
        args: vec![],
    };

    let key = cli.get_api_key("test").unwrap();
    assert_eq!(key, "cli-key");
}

#[test]
fn test_get_api_key_from_env() {
    env::set_var("TEST_API_KEY", "env-key");

    let cli = Cli {
        provider: Some("test".to_string()),
        config_file: None,
        api_key: None,
        args: vec![],
    };

    let key = cli.get_api_key("test").unwrap();
    assert_eq!(key, "env-key");

    env::remove_var("TEST_API_KEY");
}

#[test]
fn test_get_api_key_not_found() {
    env::remove_var("NONEXISTENT_API_KEY");

    let cli = Cli {
        provider: Some("nonexistent".to_string()),
        config_file: None,
        api_key: None,
        args: vec![],
    };

    let result = cli.get_api_key("nonexistent");
    assert!(matches!(result, Err(AppError::ApiKeyNotFound(_))));
}

#[test]
fn test_cli_priority_flag_over_env() {
    env::set_var("TEST_API_KEY", "env-key");

    let cli = Cli {
        provider: Some("test".to_string()),
        config_file: None,
        api_key: Some("cli-key".to_string()),
        args: vec![],
    };

    // CLI flag should take priority over env var
    let key = cli.get_api_key("test").unwrap();
    assert_eq!(key, "cli-key");

    env::remove_var("TEST_API_KEY");
}
```

## Setting Up Testing for claudio

To add testing to the claudio project:

1. **Add dev dependencies to `Cargo.toml`:**
```toml
[dev-dependencies]
tempfile = "3.13"
```

2. **Create `src/lib.rs`:**
```rust
pub mod cli;
pub mod error;
pub mod provider;
```

3. **Update `Cargo.toml` to support both lib and bin:**
```toml
[lib]
name = "claudio"
path = "src/lib.rs"

[[bin]]
name = "claudio"
path = "src/main.rs"
```

4. **Create test files:**
```bash
mkdir tests
touch tests/provider_tests.rs
touch tests/cli_tests.rs
```

5. **Run tests:**
```bash
cargo test
```

## Best Practices

1. **Test both success and error paths** - Don't just test the happy path
2. **Use descriptive test names** - `test_get_api_key_from_environment` is better than `test1`
3. **One assertion concept per test** - Keep tests focused
4. **Clean up after yourself** - Use RAII (Drop trait) for cleanup, or explicit cleanup
5. **Don't test implementation details** - Test the public API behavior
6. **Use integration tests for end-to-end flows** - Unit tests for individual functions
7. **Make tests deterministic** - Avoid randomness or timing-dependent behavior
8. **Keep tests fast** - Slow tests discourage running them frequently

## Common Pitfalls

1. **Forgetting to add `pub` to modules** - Integration tests need public APIs
2. **Not cleaning up environment variables** - Can cause test interference
3. **Using `unwrap()` in test code** - It's actually fine! Tests should panic on failure
4. **Testing private implementation details** - Focus on public behavior
5. **Overly complex test setup** - Keep it simple, or extract to helper functions

## Resources

- [The Rust Book - Testing](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Rust by Example - Testing](https://doc.rust-lang.org/rust-by-example/testing.html)
- [cargo test documentation](https://doc.rust-lang.org/cargo/commands/cargo-test.html)
- [tempfile crate](https://docs.rs/tempfile/)
