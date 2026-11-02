# Changelog

All notable changes to this project are documented here.
The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses [Semantic Versioning](https://semver.org/).

## [0.1.0] - 2026-11-02

### Added
- Hardware scan of CPU (driver, governors, EPP/EPB, turbo, HWP), platform profile, battery (health, cycles, charge-threshold support), NVMe/SATA/HDD/eMMC storage, Intel/AMD/NVIDIA GPUs, PCIe ASPM, Wi-Fi/Ethernet/Bluetooth/WWAN, USB, audio and DMI data.
- TLP configuration generator with `balanced`, `performance` and `battery` profiles and `--start-thresh` / `--stop-thresh` overrides.
- `--report` hardware report, `--install` drop-in installer, `--terse` output.
- Warnings for conflicting power daemons, missing helper tools, old TLP versions and `/etc/tlp.conf` overrides.
- Man page and bash/zsh/fish completions.
- CI, and a release pipeline producing `.tar.gz`, `.deb`, `.rpm`, `.AppImage` and Arch packages for x86_64 and aarch64, plus Nix, PKGBUILD, APKBUILD, Void, Gentoo and RPM spec recipes.
