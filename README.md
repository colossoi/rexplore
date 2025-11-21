# rexplore

A command-line tool and MCP server for exploring the public APIs of Rust crates and their dependencies.

## Requirements

**⚠️ Requires Rust nightly toolchain**

This tool depends on `cargo doc`'s JSON output format, which is only available in nightly Rust. Install nightly via:

```bash
rustup toolchain install nightly
```

You don't need to switch your default toolchain - `rexplore` will automatically use nightly when needed.

## Project Structure

This is a Cargo workspace with three crates:

- **rexplore**: Command-line interface for exploring Rust APIs
- **rexplore-core**: Library containing shared API exploration logic
- **rexplore-mcp**: MCP (Model Context Protocol) server for AI agent integration

## Installation

### CLI Tool

```bash
cargo install --path crates/rexplore
```

### MCP Server

```bash
cargo install --path crates/rexplore-mcp
```

Or install both at once:

```bash
cargo install --path crates/rexplore --path crates/rexplore-mcp
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

## MCP Server

The `rexplore-mcp` binary implements the Model Context Protocol, allowing AI agents to explore Rust crate APIs.

### Configuration

Add to your `.mcp.json`:

```json
{
  "mcpServers": {
    "rexplore-mcp": {
      "command": "/path/to/.cargo/bin/rexplore-mcp",
      "args": [],
      "transport": "stdio"
    }
  }
}
```

### Tool: `explore_crate`

The MCP server provides a single tool for exploring crate APIs:

**Parameters:**
- `manifest_path` (optional): Path to Cargo.toml (defaults to "./Cargo.toml")
- `package` (optional): Package name for workspaces
- `filter` (optional): Regex pattern to filter items
- `group_impls` (optional): Group impl blocks and condense trait impls (default: true)

**Example usage from an AI agent:**

```json
{
  "name": "explore_crate",
  "arguments": {
    "package": "tokio",
    "filter": "Runtime",
    "group_impls": true
  }
}
```

**Note:** The MCP server is LOCAL ONLY - it requires filesystem access and the Rust nightly compiler to generate rustdoc JSON.

## How It Works

The `rexplore-core` library uses `cargo rustdoc` with the nightly toolchain to generate JSON documentation, then parses and formats it into readable Rust syntax. It automatically generates `use` statements, groups related implementations, and can filter the output to help you find exactly what you need.

Both the CLI (`rexplore`) and MCP server (`rexplore-mcp`) use this shared core library to provide consistent API exploration functionality.

## Limitations

- Requires nightly toolchain (for `cargo rustdoc --output-format json`)
- Only analyzes library crates (`--lib`)
- MCP server is local-only (requires filesystem access and nightly compiler)

## License

[Add your license here]
