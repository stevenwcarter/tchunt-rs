# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
cargo build                          # build
cargo test                           # run all tests
cargo test entropy                   # run tests matching "entropy"
cargo run -- <directory>             # run the scanner on a directory
cargo install --path .               # install the binary
RUST_LOG=tchunt_rs=trace cargo run -- <dir>  # run with trace logging
just test                            # watch mode: reruns tests on file changes (requires watchexec)
```

## Architecture

**tchunt-rs** is a Rust reimplementation of the [TCHunt](https://github.com/CrowdStrike/TCHunt) utility — a forensic tool that identifies potentially encrypted or compressed files by detecting high Shannon entropy.

### Detection logic (`src/lib.rs`)

`search_dir` walks a directory recursively (async, via `async-walkdir`) and calls `check_file` on each entry. `check_file` applies these filters in order:

1. Skip directories and files whose size isn't divisible by 512 or is under 2KB
2. Compute Shannon entropy via `entropy::Entropy` — skip if < 7.93
3. Use the `infer` crate to detect known file types — skip recognized types (print them at TRACE level instead)
4. Print remaining files (unrecognized type + high entropy) to stdout as candidates

### Entropy calculation (`src/entropy.rs`)

`Entropy` is a generic struct over an async reader (`tokio::fs::File` or `Cursor<Vec<u8>>`). The `shannon()` method reads up to 171,072 bytes from the start of the file, and if the file is large enough, also reads 171,072 bytes from the end, then computes Shannon entropy over the combined byte frequency counts. This sampling approach avoids reading entire large files.

### Test resources

`test-resources/` contains binary fixtures used for integration testing:
- `random512` / `random65536` — random bytes (high entropy)
- `zero512` — zero bytes (low entropy)
