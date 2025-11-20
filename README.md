# rexplore

A command-line tool for exploring the public APIs of Rust crates and their dependencies.

## Requirements

**⚠️ Requires Rust nightly toolchain**

This tool depends on `cargo doc`'s JSON output format, which is only available in nightly Rust. Install nightly via:

```bash
rustup toolchain install nightly
```

You don't need to switch your default toolchain - `rexplore` will automatically use nightly when needed.

## Installation

```bash
cargo install --path .
```

## Features

- **Explore any dependency**: Analyze the public API of any crate in your dependency tree
- **Exact version matching**: See the API as your project actually uses it, not the latest docs
- **Smart filtering**: Find specific items using keyword search or regex patterns
- **Clean output**: Formats APIs as readable Rust code with automatic `use` statements
- **Impl grouping**: Organizes related implementations together for better readability
- **No internet required**: Works entirely offline using your local dependencies

## Usage

### Explore a Dependency

View the complete public API of a crate in your project:

```bash
rexplore --package anyhow
```

Output includes organized `use` statements and all public items:

```rust
use anyhow::{Context, Error, Result};

pub fn bail<T>(message: T) -> Result<T>;
pub struct Error;
pub trait Context<T> {
    fn context<C>(self, context: C) -> Result<T>;
}
```

### Filter by Keyword

Find items containing a specific substring:

```bash
rexplore --package tokio --keyword Runtime
```

Shows only items with "Runtime" in their definition:

```rust
pub struct Runtime;
impl Runtime {
    pub fn new() -> std::io::Result<Runtime>;
    pub fn block_on<F>(&self, future: F) -> F::Output;
}
```

### Filter by Regex

Use pattern matching for more precise filtering:

```bash
rexplore --package serde --regex "^pub trait (Serialize|Deserialize)"
```

Finds all traits starting with Serialize or Deserialize.

### Explore Current Project

Analyze your own crate's public API:

```bash
rexplore
```

Great for reviewing what you're exposing publicly before publishing.

### Custom Project Path

Explore a crate in a different workspace:

```bash
rexplore --manifest-path ../other-project/Cargo.toml --package clap
```

## Command-Line Options

```
Options:
  --manifest-path <PATH>    Path to Cargo.toml [default: Cargo.toml]
  -p, --package <NAME>      Package name to analyze
  -k, --keyword <KEYWORD>   Filter by substring match (mutually exclusive with --regex)
  -r, --regex <PATTERN>     Filter by regex pattern (mutually exclusive with --keyword)
  -h, --help                Print help
  -V, --version             Print version
```

## Use Cases

**Dependency exploration**: Quickly understand what a new dependency offers without reading source code or rustdocs.

**Version-specific APIs**: Check if a specific feature exists in the exact version you're using, not just the latest docs.

**API auditing**: Review your own crate's public surface before publishing to ensure you're not exposing internals.

**Finding specific types**: Locate specific traits, structs, or functions across large crates using filters.

**Migration planning**: Compare different versions of a dependency by switching branches and running rexplore.

## Example Workflows

### "What does this error type expose?"

```bash
rexplore -p anyhow -k Error
```

### "What async runtime types are available?"

```bash
rexplore -p tokio -r "Runtime|Executor"
```

### "What traits can I implement?"

```bash
rexplore -p serde -r "^pub trait"
```

### "Review my public API before publishing"

```bash
rexplore
```

## How It Works

`rexplore` uses `cargo rustdoc` with the nightly toolchain to generate JSON documentation, then parses and formats it into readable Rust syntax. It automatically generates `use` statements, groups related implementations, and can filter the output to help you find exactly what you need.

## Limitations

- Requires nightly toolchain (for `cargo doc --output-format json`)
- Only analyzes library crates (`--lib`)
- Does not support private items (`--document-private-items`)

## License

[Add your license here]
