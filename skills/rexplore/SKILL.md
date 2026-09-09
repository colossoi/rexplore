---
name: rexplore
description: Inspect the public Rust API for the exact crate version selected by a Cargo project. Use when discovering available types, traits, functions, methods, or impls, checking a dependency's version-specific API, or auditing a library crate's public surface. Do not use for private implementation details or runtime behavior.
---

# Rexplore

Use the `rexplore` CLI to answer questions from generated rustdoc JSON. Report the relevant API declarations and explain only the details needed for the user's question.

## Select the target

- Run from the Cargo project or workspace root whose dependency resolution should be inspected.
- Pass `--manifest-path <path-to-Cargo.toml>` when the target manifest is not `./Cargo.toml`.
- Pass `--package <name>` for a dependency or a specific workspace member. Omit it to inspect the manifest's own library crate.
- Prefer `--keyword <text>` for one literal name and `--regex <pattern>` for alternatives or declaration shapes. The two filters are mutually exclusive.
- Start with a focused filter. Request the complete API only when the question genuinely needs the whole surface.

## Invoke the CLI

Invoke the installed binary directly:

```text
rexplore --manifest-path ./Cargo.toml --package <crate> --keyword <name>
```

For broader matching, replace `--keyword <name>` with `--regex <pattern>`. Quote regex patterns according to the active shell.

If `rexplore` is unavailable on `PATH`, report that the CLI must be installed and stop.

## Interpret results

- Treat the output as the public surface of the exact version resolved by the target Cargo project, including enabled-feature effects.
- Preserve signatures, bounds, associated items, and impl ownership when they matter to the answer.
- If no item matches, retry once with a less restrictive filter or the unqualified item name before concluding it is absent.
- Distinguish absence from generation failure. Rexplore requires `rustup`, a nightly Rust toolchain, and a library target; surface missing-toolchain, Cargo-resolution, and rustdoc errors directly.
