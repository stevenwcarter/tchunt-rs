# Deferred Issues

Lower-priority code quality issues identified during code review, deferred from the main fix plan.

---

## 11. `Entropy<R>` generic adds complexity for minimal benefit

The `Entropy` struct uses a generic type parameter `R` that is only ever instantiated with `tokio::fs::File` or `Cursor<Vec<u8>>`. The generic adds type-level complexity (impl blocks, trait bounds) that may be surprising to contributors. Consider simplifying to an enum or removing the `Cursor` variant entirely (it's only used in tests — the tests could open real files from `test-resources/` instead).

**File:** `src/entropy.rs`

---

## 12. `println!` mixed with `tracing` macros for output

`check_file` uses `println!` to output detected files but uses `trace!`/`error!` for other messages. This means the output of detected files cannot be filtered or redirected using `RUST_LOG`. It also makes it harder to suppress output during testing. Consider using a dedicated output mechanism or accepting that `println!` is the "result" channel and all diagnostic messages use tracing.

**File:** `src/lib.rs`, line 65

---

## 13. `infer::get_from_path` reads the file a second time

After `Entropy` reads up to 342KB of the file to compute entropy, `infer::get_from_path(filename)` opens the file again to read its magic bytes. This is a redundant I/O operation for every high-entropy file. Consider using `infer::get(&first_bytes)` with the buffer already read during entropy calculation, passing it out of `Entropy` or reading it separately before constructing `Entropy`.

**File:** `src/lib.rs`, line 58

---

## 14. No validation that the directory argument exists

`main.rs` checks that an argument was provided but doesn't validate that the path exists or is a directory. `WalkDir` will silently produce no results (or a single error entry) if given a nonexistent path. A simple `tokio::fs::metadata(&args[1]).await` check with a user-friendly error message would improve the UX significantly.

**Files:** `src/main.rs`, `src/lib.rs`
