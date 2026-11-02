# Repository Guidelines

## Project Structure & Module Organization

This proof-of-concept SQL database uses a Rust 2024 Cargo workspace with two library crates:

- `storage/`: tuple and value representations; table and constraint APIs are planned.
- `engine/`: execution-layer scaffold; depends on storage; upstream `sqlparser` integration is planned.
- `docs/design/`: architecture and SQL grammar documentation, especially `sql_parsing.md`.
- `docs/usage/`: usage documentation.

Keep lexer, parser, AST, storage, and execution responsibilities in their corresponding modules.

## Build, Test, and Development Commands

Run commands from the repository root with a Rust toolchain supporting edition 2024:

- `cargo build --workspace`: compile both crates.
- `cargo check --workspace`: quickly check types and compilation.
- `cargo test --workspace`: run workspace unit, integration, and documentation tests.
- `cargo fmt --all -- --check`: check standard Rust formatting; use `cargo fmt --all` to apply it.
- `cargo clippy --workspace --all-targets`: check Rust lint diagnostics.

The workspace currently contains libraries; there is no application binary to launch with `cargo run`.

## Coding Style & Naming Conventions

Use rustfmt formatting with four-space indentation. Use `snake_case` for modules, functions, and variables; `PascalCase` for structs and enums; and `SCREAMING_SNAKE_CASE` for constants. Use upstream `sqlparser` for planned SQL parsing and keep its syntax AST internal to engine preparation. Keep unrelated formatting changes out of focused patches.

## Testing Guidelines

Tests use Rust's built-in `#[test]` framework. Name behavior tests descriptively, following examples such as `test_create_table_basic`. When SQL preparation is implemented, add tests for accepted SQL, unsupported features, malformed input, and parameter handling. Keep SQL design documentation synchronized with supported behavior. No numeric coverage threshold is configured.

## Commit & Pull Request Guidelines

Recent history favors descriptive, prefixed subjects such as `docs: refine SQL query grammar`; follow that pattern with an appropriate change type. Keep commits focused. Pull requests should explain the behavior changed, reference relevant issues when available, and report validation commands and results. For SQL changes, include representative queries and describe grammar or AST effects.
