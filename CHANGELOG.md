# Changelog

All notable changes to this project are documented here.
The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses [Semantic Versioning](https://semver.org/).

## [0.2.0] - 2026-11-07

### Added
- `--diff` shows what would change compared with the installed config (`--against FILE` compares with any file). The exit status is 1 when there are differences, so it works in scripts and cron jobs.
- `--uninstall` removes the drop-in, and `--apply` runs `tlp start` after `--install` or `--uninstall`.
- `--only` / `--skip` generate a partial config (sections: general, cpu, platform, graphics, storage, pcie, radios, usb, audio, kernel, battery).
- `--format nix` writes a NixOS `services.tlp.settings` snippet, `--format json` a flat JSON object for scripts.
- `-o -` writes to standard output.
- `--sysroot DIR` (or `AUTO_TLP_SYSROOT`) scans a directory tree instead of the live system. It never runs host commands and refuses `--install`/`--uninstall`.
- GitHub release pages now show the matching CHANGELOG section; a release without one fails early.
- 23 end-to-end tests that run the real binary against fake machines (Intel + hybrid NVIDIA ThinkPad, AMD desktop, SATA/NVMe/HDD, one or two batteries, missing TLP, garbage sysfs values, an empty machine), 95 tests in total.

### Fixed
- Several identical external USB devices (for example two of the same mouse) produce a single `USB_DENYLIST` entry.
- Intel GPUs on newer kernels that expose frequencies under `gt/gt0/rps_*` get correct frequency limits.
- `--help` is grouped into sections and its `--profile <NAME>` line is aligned.

## [0.1.0] - 2026-11-02

### Added
- Hardware scan of CPU (driver, governors, EPP/EPB, turbo, HWP), platform profile, battery (health, cycles, charge-threshold support), NVMe/SATA/HDD/eMMC storage, Intel/AMD/NVIDIA GPUs, PCIe ASPM, Wi-Fi/Ethernet/Bluetooth/WWAN, USB, audio and DMI data.
- TLP configuration generator with `balanced`, `performance` and `battery` profiles and `--start-thresh` / `--stop-thresh` overrides.
- `--report` hardware report, `--install` drop-in installer, `--terse` output.
- Warnings for conflicting power daemons, missing helper tools, old TLP versions and `/etc/tlp.conf` overrides.
- TLP-version-aware output: TLP 1.11+ gets the new `_ON_PRF` / `_ON_BAL` / `_ON_SAV` key names (including TLP's power-saver profile), older or undetected TLP gets the classic `_ON_AC` / `_ON_BAT` names. Deprecated or removed settings (`TLP_PERSISTENT_DEFAULT`, `RADEON_DPM_STATE_*`) are never written for new TLP.
- `--tlp-version X.Y` to generate for another TLP version, `--demo` to preview a full-featured sample laptop.
- TLP version detection from TLP's own files (`tlp-func-base`, `tlp-stat`).
- Man page and bash/zsh/fish completions.
- CI that also validates every generated key against a real TLP (installed and built from upstream source), and a release pipeline producing `.tar.gz`, `.deb`, `.rpm`, `.AppImage` and Arch packages for x86_64 and aarch64, plus Nix, PKGBUILD, APKBUILD, Void, Gentoo and RPM spec recipes.
