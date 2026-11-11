---
name: rust-developer
description: Write, review, debug, and refactor idiomatic, production-quality Rust. Use this skill whenever the task involves Rust in any way — writing new Rust code or crates, fixing borrow-checker or lifetime errors, designing traits and public APIs, error handling, async/Tokio, concurrency, unsafe code, FFI, performance tuning, Cargo.toml / workspace setup, tests and benchmarks, or porting code from another language to Rust. Trigger even when the user only pastes a compiler error, mentions `cargo`, `.rs` files, crates, `clippy`, or asks "how would I do X in Rust", and even for small snippets — the conventions here prevent the most common mistakes.
---

# Rust Developer

This skill helps produce Rust that compiles cleanly, passes `clippy`, reads like idiomatic Rust written by an experienced developer, and holds up in production. Rust rewards getting the design right early: ownership, error types, and trait boundaries are hard to change later, so think about them before writing large amounts of code.

## Core workflow

Follow this loop for any non-trivial change. The compiler is the fastest feedback you have, so lean on it constantly rather than writing a lot of code and compiling once at the end.

1. **Understand the context.** Read `Cargo.toml` (edition, MSRV via `rust-version`, dependencies, features) and skim `src/lib.rs` / `src/main.rs` and module layout. Match existing conventions (error crate, async runtime, logging crate, naming) instead of introducing competing ones.
2. **Design types first.** Decide who owns each piece of data, what the error type is, and which functions borrow vs. take ownership. Sketch signatures before bodies.
3. **Write in small increments**, running `cargo check` after each meaningful step.
4. **Verify before declaring done:**
   ```bash
   cargo fmt --all
   cargo clippy --all-targets --all-features -- -D warnings
   cargo test --all-features
   cargo doc --no-deps   # for libraries: catches broken intra-doc links
   ```
5. **Explain trade-offs** to the user when you made a non-obvious choice (e.g. cloning to sidestep a lifetime, `Arc<Mutex<_>>` vs. a channel, `Box<dyn Trait>` vs. generics).

If a toolchain isn't available in the environment, try `rustup` or the system package manager (`apt install cargo rustc`). If neither works, say clearly that the code is unverified, and be extra careful with borrows and lifetimes.

## Project setup

- Use the latest stable edition (`edition = "2024"`) for new crates unless the project pins an older one.
- Set `rust-version` for libraries so downstream users know the MSRV.
- Library vs. binary: put logic in `lib.rs` and keep `main.rs` thin (parse args, set up logging, call into the library). This makes the logic testable.
- Workspaces: share dependency versions with `[workspace.dependencies]` and lints with `[workspace.lints]`.
- Recommended lint baseline in `Cargo.toml`:
  ```toml
  [lints.rust]
  unsafe_code = "forbid"          # relax to "deny" + targeted allows if unsafe is genuinely needed
  missing_docs = "warn"           # libraries only

  [lints.clippy]
  all = { level = "warn", priority = -1 }
  pedantic = { level = "warn", priority = -1 }
  module_name_repetitions = "allow"
  must_use_candidate = "allow"
  ```
- Add dependencies with `cargo add <crate>` (and `--features` as needed) rather than hand-editing versions from memory, so you get a current, real version. Use `default-features = false` when you only need a subset.

## Ownership and borrowing

Pick parameter types by what the function actually needs:

| Need | Parameter type |
|---|---|
| Read a string | `&str` (not `&String`) |
| Read a slice | `&[T]` (not `&Vec<T>`) |
| Store/consume a value | `T` by value (caller decides whether to clone) |
| Accept many string-like inputs, then store | `impl Into<String>` |
| Read path-like input | `impl AsRef<Path>` |
| Maybe-owned return | `Cow<'_, str>` |

Guidelines:

- Return owned values from constructors and transformations; return borrows from accessors.
- `clone()` is fine when it simplifies code and the data is small or cloned rarely. It is a smell inside hot loops or when used to silence the borrow checker without understanding why. If you clone to work around a borrow error, mention it.
- Prefer restructuring over fighting the borrow checker: split a struct so disjoint fields can be borrowed separately, shorten borrow scopes with blocks, collect indices first then mutate, or use `std::mem::take` / `replace` to move out of `&mut`.
- Reach for `Rc<RefCell<T>>` (single-threaded) or `Arc<Mutex<T>>` (multi-threaded) only when ownership is genuinely shared. Graph-like structures are often cleaner as an arena (`Vec<Node>` + index IDs).

### Lifetimes

- Let elision work; add explicit lifetimes only when the compiler asks or when it clarifies an API.
- A struct holding a reference (`struct Parser<'a> { input: &'a str }`) is fine for short-lived views. For long-lived data, own it instead — lifetimes on long-lived structs spread through the whole codebase.
- `'static` in a trait bound (`T: 'static`) means "contains no non-static borrows," not "lives forever." Owned types like `String` satisfy it.
- Never fix a lifetime error by leaking memory (`Box::leak`) unless the value truly lives for the program's duration.

## Error handling

Errors are part of the API. Choose deliberately:

- **Libraries:** define a concrete error enum, usually with `thiserror`. Callers can match on variants. Don't expose `anyhow::Error` from a library's public API.
  ```rust
  #[derive(Debug, thiserror::Error)]
  pub enum ConfigError {
      #[error("config file not found at {path}")]
      NotFound { path: PathBuf },
      #[error("invalid TOML in config")]
      Parse(#[from] toml::de::Error),
      #[error(transparent)]
      Io(#[from] std::io::Error),
  }
  ```
- **Applications/binaries:** `anyhow::Result<T>` with `.context("what we were doing")` at each meaningful layer, so the final error reads like a story.
- Use `?` for propagation. Avoid `.unwrap()` and `.expect()` in library code and in any path reachable from user input. `.expect("reason")` is acceptable for true invariants — write the message as the invariant ("regex is valid at compile time").
- Tests, examples, and prototypes may use `unwrap()`; tests can also return `Result<(), Box<dyn Error>>`.
- Don't use `panic!` for recoverable conditions. Do document panics in a `# Panics` doc section when a public function can panic.
- Use `Option` for "absence is normal," `Result` for "something went wrong."

## Idiomatic code

- **Iterators over index loops.** `iter().map().filter().collect()` is usually clearer and equally fast. Use `for` loops when there are side effects or early exits that make a chain awkward.
- **Combinators:** `map`, `and_then`, `ok_or_else`, `unwrap_or_default`, `is_some_and`, `let ... else`, and `if let` chains keep control flow flat.
  ```rust
  let Some(user) = users.get(id) else {
      return Err(Error::UnknownUser(id));
  };
  ```
- **Pattern matching:** prefer exhaustive `match` over chains of `if`; avoid `_ =>` on enums you own so adding a variant forces you to handle it.
- **Newtypes** for domain values (`struct UserId(u64);`) prevent mixing up arguments and let you attach invariants.
- **Make invalid states unrepresentable:** use enums with data instead of a struct with several optional fields and a "kind" flag.
- **Derive generously:** `Debug` on almost everything; `Clone, PartialEq, Eq, Hash, Default` when meaningful; `serde::{Serialize, Deserialize}` behind a `serde` feature for libraries.
- **Conversions:** implement `From` (you get `Into` for free), `TryFrom` for fallible conversions, `FromStr` for parsing, `Display` for user-facing text.
- **Builders** for structs with many optional settings; `Default` + struct update syntax for simple config.
- Use `#[must_use]` on functions whose return value is easy to ignore by mistake.
- Naming: `snake_case` functions/modules, `CamelCase` types, `SCREAMING_SNAKE_CASE` constants; conversions follow `as_` (cheap borrow), `to_` (expensive/owned), `into_` (consumes self).

## Traits and generics

- Generics (`fn f<T: Trait>(x: T)` / `impl Trait`) give static dispatch and inlining; trait objects (`&dyn Trait`, `Box<dyn Trait>`) give smaller binaries, heterogeneous collections, and faster compiles. Default to generics for small hot functions, trait objects for plugin-style extensibility.
- Keep bounds on `impl` blocks and functions, not on struct definitions, unless the struct can't exist without them.
- Prefer small, focused traits. Provide default methods where sensible.
- Use the sealed-trait pattern if downstream crates shouldn't implement a public trait.
- Implement standard traits (`Iterator`, `IntoIterator` for `&T` and `T`, `Extend`, `FromIterator`, `AsRef`) so your types work with the ecosystem.
- Avoid premature abstraction: write the concrete version first and generalize when a second use appears.

## Concurrency

- The compiler enforces data-race freedom via `Send` and `Sync`; if a type isn't `Send`, find out why (`Rc`, `RefCell`, raw pointers) before wrapping it in something.
- Prefer, roughly in this order: no sharing (move data into threads) → message passing (`std::sync::mpsc`, `crossbeam-channel`) → `Arc<RwLock<T>>` / `Arc<Mutex<T>>` → atomics.
- Use `std::thread::scope` to borrow local data in threads without `Arc`.
- Data parallelism over collections: `rayon` (`par_iter()`) is usually the simplest correct answer.
- Keep lock scopes short; never hold a lock while doing I/O or calling unknown code. Watch for deadlocks from acquiring multiple locks in inconsistent order.
- `parking_lot` locks are a fine drop-in if the project already uses them.

## Async

- Use the runtime the project already uses (most commonly Tokio). Don't mix runtimes.
- **Never block inside async code.** No `std::thread::sleep`, synchronous file/network I/O, or long CPU work on the executor. Use `tokio::time::sleep`, async I/O, or `tokio::task::spawn_blocking` for CPU-heavy or blocking calls.
- Don't hold a `std::sync::Mutex` guard across `.await` (it makes the future `!Send` and risks deadlocks). Either scope the guard so it drops before the await or use `tokio::sync::Mutex` when you truly must hold it across awaits.
- Spawned tasks need `'static + Send` futures: move owned data in, clone `Arc`s before `spawn`.
- Handle cancellation: any `.await` can be the last line that runs. Be careful with `select!` on futures that aren't cancel-safe.
- Use `JoinSet` or `futures::stream::FuturesUnordered` for many concurrent tasks; bound concurrency with a `Semaphore` or `buffer_unordered(n)`.
- Async functions in traits are supported natively; reach for `async-trait` only when you need `dyn` dispatch on older patterns or the project already uses it.
- Add timeouts (`tokio::time::timeout`) around network calls.

## Unsafe code and FFI

- Treat `unsafe` as a last resort. First look for a safe std API or a well-reviewed crate.
- When it's necessary: keep `unsafe` blocks minimal, wrap them in a safe abstraction, and put a `// SAFETY:` comment directly above each block explaining why every invariant holds. Public `unsafe fn`s need a `# Safety` doc section listing caller obligations.
- Enable `#![deny(unsafe_op_in_unsafe_fn)]` (default in edition 2024) so unsafe operations stay explicit.
- Test unsafe code under Miri (`cargo +nightly miri test`) where possible.
- FFI: use `bindgen` for C headers, `cbindgen` to export headers, `#[repr(C)]` on shared structs, `CStr`/`CString` for strings, and never let a panic unwind across an `extern "C"` boundary (catch it or use `extern "C-unwind"` deliberately).

## Performance

Measure before optimizing. Most Rust code is fast enough by default; clarity wins until a profiler says otherwise.

- Benchmark with `criterion` or `divan`; profile with `cargo flamegraph`, `perf`, or `samply`. Always benchmark `--release`.
- Common wins: avoid repeated allocation in loops (reuse buffers, `Vec::with_capacity`), avoid needless `clone`/`to_string`, use `&str` slices instead of building `String`s, iterate rather than index, prefer `HashMap` with a faster hasher (`ahash`, `rustc-hash`) for non-adversarial keys, use `BufReader`/`BufWriter` for I/O.
- `Box<[T]>` / `Arc<str>` for immutable data that's shared or stored long-term.
- Release profile tweaks when size or speed matters: `lto = "thin"`, `codegen-units = 1`, `panic = "abort"` (only if nothing relies on unwinding).
- Don't sprinkle `#[inline]` everywhere; it's mainly useful for small functions called across crate boundaries.

## Testing

- **Unit tests** live in the same file under `#[cfg(test)] mod tests { use super::*; ... }` and can test private items.
- **Integration tests** in `tests/` exercise only the public API — that's the point; it catches API awkwardness.
- **Doc tests**: examples in `///` comments are compiled and run. Every public item of a library should ideally have one. Use `?` in examples with a hidden `# Ok::<(), Box<dyn std::error::Error>>(())` line.
- **Property tests** with `proptest` or `quickcheck` for parsers, encoders, and anything with round-trip or algebraic properties.
- **Snapshot tests** with `insta` for complex output.
- Async tests: `#[tokio::test]`.
- Test error paths, not just happy paths. Name tests for the behavior: `rejects_empty_username`, not `test1`.
- `cargo nextest run` is a faster runner if the project uses it.

## Documentation

- Crate-level docs (`//!` in `lib.rs`) should say what the crate does and show a quick-start example.
- Public items get `///` docs: one-line summary, blank line, details. Use standard sections — `# Examples`, `# Errors`, `# Panics`, `# Safety`.
- Link with intra-doc links: ``[`Config::load`]``.
- Comments in code explain *why*, not *what*.

## Common compiler errors and fixes

- **E0382 use of moved value** — borrow (`&x`) instead of moving, clone if cheap, or restructure so the value is used once. In loops, `for x in &vec` instead of `for x in vec`.
- **E0499 / E0502 conflicting borrows** — shorten the first borrow's scope, split the struct, compute what you need into a local first, or use indices.
- **E0505 move out while borrowed** — the borrow is still alive; end it (drop the reference) before moving.
- **E0597 borrowed value does not live long enough** — the referent is dropped too early; move it to an outer scope or return an owned value.
- **E0106 missing lifetime specifier** — usually a sign the function should return an owned type, or the returned reference must tie to a specific input: `fn first<'a>(a: &'a str, b: &str) -> &'a str`.
- **E0277 trait bound not satisfied** — add the bound, derive the trait, or (for `?`) add a `From` impl / `.map_err(...)`.
- **"future cannot be sent between threads safely"** — a non-`Send` value (`Rc`, `RefCell`, `MutexGuard`) is alive across an `.await`; drop it before the await or switch types.
- **E0308 mismatched types with `impl Trait`** — different branches return different concrete types; use `Box<dyn Trait>`, an enum, or `either::Either`.
- **"cannot borrow as mutable" on a closure** — the closure needs `FnMut` or must be declared `let mut f = ...`.

When fixing an error the user pasted, explain the cause in one or two sentences, then give the fix. Don't just make it compile; make sure the fix preserves the intended behavior.

## Crate recommendations

Prefer what the project already uses. Otherwise these are well-established defaults (check current versions with `cargo add` or crates.io rather than from memory):

- Errors: `thiserror` (libraries), `anyhow` or `eyre` (applications)
- Serialization: `serde` + `serde_json` / `toml` / `serde_yaml`-style format crates
- CLI: `clap` (derive API)
- Async: `tokio`; HTTP client `reqwest`; HTTP server `axum`
- Logging/tracing: `tracing` + `tracing-subscriber`
- Parallelism: `rayon`; channels: `crossbeam-channel`
- Testing: `proptest`, `insta`, `criterion`
- Database: `sqlx` (async, compile-time checked queries) or `diesel`
- Time: `jiff` or `chrono`; regex: `regex`

Be cautious adding dependencies to libraries: each one becomes a burden for every downstream user. Run `cargo audit` / `cargo deny` in projects that care about supply chain.

## Anti-patterns to avoid

- `.unwrap()` scattered through library code.
- `&String`, `&Vec<T>`, or `&Box<T>` as parameters.
- `Rc<RefCell<_>>` / `Arc<Mutex<_>>` everywhere as a reflex to avoid thinking about ownership.
- Stringly-typed APIs (`kind: &str`) where an enum belongs.
- Returning `Result<T, String>` or `Box<dyn Error>` from library APIs.
- Writing Java/C++-style inheritance hierarchies with traits; use composition and enums.
- Implementing `Deref` to fake inheritance.
- Blocking calls inside async functions.
- `#[allow(...)]` to silence clippy without a comment explaining why.
- Massive `mod.rs` / `lib.rs` files; split modules by responsibility.

## Porting from other languages

- Don't translate line by line. Re-model the data around ownership: who creates it, who mutates it, who reads it.
- Class hierarchies → enums (closed set) or traits (open set).
- Null → `Option`; exceptions → `Result`; callbacks with captured mutable state → closures with explicit ownership or channels.
- Global mutable state → pass context explicitly, or `OnceLock`/`LazyLock` for truly global read-mostly data.

## Review checklist

Before handing code back, check:

- [ ] `cargo fmt`, `cargo clippy -- -D warnings`, and `cargo test` pass (or you've said they couldn't be run).
- [ ] No `unwrap`/`expect` on fallible paths reachable from input, outside tests.
- [ ] Error types fit the crate kind (concrete enum for libs, `anyhow` + context for apps).
- [ ] Function parameters borrow where possible and take ownership only when storing.
- [ ] No blocking in async; no lock guards held across `.await`.
- [ ] Every `unsafe` block has a `// SAFETY:` comment.
- [ ] Public items are documented, with examples for the main entry points.
- [ ] Tests cover both success and failure cases.
- [ ] New dependencies are justified and use minimal features.
