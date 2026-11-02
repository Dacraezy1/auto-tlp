## What does this change?

## Checklist
- [ ] `cargo fmt --all` and `cargo clippy --all-targets -- -D warnings` are clean
- [ ] `cargo test` passes (new behaviour has a test in `src/gen.rs` or `tests/cli.rs`)
- [ ] New TLP keys are only emitted when the hardware supports them
- [ ] README / man page updated if the CLI or the supported settings changed
