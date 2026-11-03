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
| `src/demo.rs` | Built-in sample laptop used by `--demo`, CI and the unit tests. |
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
4. If you changed output, regenerate the examples:
   ```console
   cargo build --release
   for pair in 1.11:example-config.conf 1.6:example-config-classic.conf; do
     ./target/release/auto-tlp --demo --tlp-version "${pair%%:*}" | sed 's/^# Generated : .*/# Generated : 2026-11-03 10:00 UTC/' > "docs/${pair##*:}"
   done
   ```
5. Emit settings through `w.dual("BASE", ac, bat, sav)` so they get the right names for old and new TLP.
   Add new keys that have a power-saver variant in upstream TLP to `SAV_KEYS` in `src/gen.rs`.
6. Check against a real TLP: `scripts/validate-keys.sh` (needs TLP installed) and
   `scripts/check-with-tlp-source.sh` (builds upstream TLP, needs git and make).

By contributing you agree that your work is licensed under GPL-3.0-or-later.
