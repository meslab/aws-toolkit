# AGENTS.md

Single Rust crate (`aws-toolkit`): a library (`src/lib.rs`) plus six CLI binaries.
Service logic lives in `src/<service>.rs`; `src/<bin>.rs` files are thin clap wrappers.

## Commands

CI is the source of truth (`.github/workflows/ci.yml`):

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
```

- Run one test: `cargo test matches_filters` or `cargo test --bin ses-suppression-list`.
- `make lint` is only `cargo clippy`; it does **not** enforce CI's `--all-targets -- -D warnings`. Use the CI command before finishing.
- `make build` runs `cargo update` first and rewrites `Cargo.lock`. Prefer `cargo build` unless a dependency refresh is intended.
- `make upgrade` removes/re-adds every `aws-*` dependency and rewrites `Cargo.toml`; do not run it as part of routine work.
- `make release` builds and strips all six release binaries. `make install` copies them to `~/.local/bin`.
- `cargo run` runs `ssm-session` (`default-run`); use `cargo run --bin <name>` for another binary.

## Architecture and conventions

- Every AWS client goes through `initialize_client::<SdkClient>(Region, profile)` in `src/client.rs`, which uses the standard `DefaultCredentialsChain`.
- Adding an AWS service requires: the `aws-sdk-*` dependency with `features = ["behavior-version-latest"]`, an `impl_aws_client_builder!` entry in `src/client.rs`, and the module in `src/lib.rs`.
- Adding a binary requires updating **all** of: `Cargo.toml` `[[bin]]`, `src/lib.rs`, `Makefile` (`release`, `all`, `install`, `uninstall`, `.PHONY`), and `.docker/dockerfile`. Missing one means it silently is not built or installed.
- Errors use `AppResult<T> = Result<T, Box<dyn Error>>` from `src/errors.rs`; propagate with `?`/`.into()`. Do not add `anyhow`/`thiserror`.
- Every `main` calls `env_logger::init()`. Use `log::{debug,info}` for diagnostics (`RUST_LOG=debug`); use `println!`/`eprintln!` for user output.
- CLI defaults are `--region eu-central-1` and `--profile default`.
- Listing filters are `-m/--prefix-match` and `-x/--prefix-exclude`, implemented by `matches_filters` in `src/utils.rs`: case-sensitive substring `contains`, empty include matches everything, exclude wins. Reuse it rather than adding AWS-side name filters.
- Paginate AWS list calls: `.into_paginator().send()` where available, otherwise the explicit `next_token` loop pattern (see `src/guardduty.rs`). Avoid one-shot list calls that truncate results.
- `src/utils.rs` is a private module (`mod utils`); `sanitize_string` is re-exported as `pub(crate)` from `src/lib.rs`. Put internal helpers there.
- Toolchain is Rust 1.88+ with edition 2024; the code uses let-chains (`if let ... && let ...`, `if ... && let ...`). Do not rewrite these into nested blocks.

## Tests and operational safety

- Tests are only unit tests in `src/utils.rs` and `src/ses-suppression-list.rs`. There are no integration tests, AWS mocks, or local emulators; `cargo test` needs no credentials.
- Binaries call real AWS APIs. Only `release-codepipelines` and `s3-guardduty-copy` support `--dry-run`.
- `scale-in-ecs --delete` terminates EC2 instances and deletes NAT gateways, load balancers, ECS services, RDS, and ElastiCache resources. Confirmation is requested only when `--delete` is combined with `--skip-final-rds-snapshot`; `--delete` alone proceeds without prompting. Never run it without explicit user approval.
- `ssm-session` shells out to the external `aws ssm start-session` CLI, so the AWS CLI must be installed.

## Git commits

- Keep the commit message to a single line of at most 72 characters, counting the `<branch>: ` prefix that the author's global `prepare-commit-msg` hook adds.
- Never add `Co-Authored-By:` or any other attribution trailer.
