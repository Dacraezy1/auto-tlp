# Contributing

Thanks for helping! The most valuable contributions are **hardware reports** and **fixes for misdetected devices**.

## Reporting a problem

Open an issue and paste the output of:

```console
auto-tlp --report --verbose
```

Say which generated setting is wrong and what you expected. Include `tlp --version` and your distro.

## Development

```console
cargo build
cargo test                                          # unit + end-to-end tests
cargo fmt --all
cargo clippy --all-targets -- -D warnings           # CI enforces both (Rust 1.75)
```

The project deliberately has **no crate dependencies**. Please keep it that way.

### How the code is organised

| File | Role |
|---|---|
| `src/hw.rs` | Detection only. Reads sysfs/procfs/DMI into plain structs. No TLP knowledge here. |
| `src/gen.rs` | Decisions only. Turns the structs into TLP keys. No file access here, so it is fully unit-testable with fixtures. |
| `src/report.rs` | Human-readable `--report`. |
| `src/main.rs` | CLI parsing, `--install`, warnings output. |

### Rules of thumb

1. **Only emit a key if the hardware supports it.** Add a test proving it is absent when the hardware is absent.
2. **Only emit values the kernel offers** (check the `*_available_*` lists).
3. Detection must never write anywhere and must tolerate missing files (`Option` everywhere).
4. Gate keys that depend on the TLP version with `hw.tlp.at_least(major, minor)`.
5. New behaviour needs a test in `src/gen.rs` (use the `laptop()` fixture) and, if it is user-facing, a README/man-page update.

### Adding support for a new device

1. Add the detection to `src/hw.rs` and a field on the right struct.
2. Add the decision to `src/gen.rs` (one function per section).
3. Add fixtures/tests, run `cargo test`.
4. If you changed output, regenerate the example:
   `cargo test print_example -- --ignored --nocapture` (copy the config part into `docs/example-config.conf`).

By contributing you agree that your work is licensed under GPL-3.0-or-later.
