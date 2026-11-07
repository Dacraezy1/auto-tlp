<div align="center">

# auto-tlp

**Scan your Linux machine. Get a TLP config that fits it exactly.**

[![CI](https://github.com/Dacraezy1/auto-tlp/actions/workflows/ci.yml/badge.svg)](https://github.com/Dacraezy1/auto-tlp/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/Dacraezy1/auto-tlp)](https://github.com/Dacraezy1/auto-tlp/releases/latest)
[![License: GPL v3+](https://img.shields.io/badge/license-GPL--3.0--or--later-blue.svg)](LICENSE)
![Rust 1.75+](https://img.shields.io/badge/rust-1.75%2B-orange)

</div>

[TLP](https://linrunner.de/tlp) is the best set-and-forget power manager for Linux laptops, but its default
config is generic, and tuning it by hand means checking your CPU driver, your battery's charge-limit support,
your disks, your GPU... **auto-tlp does that homework for you.** It reads your hardware, then writes a clean
config that contains only the settings *your* machine actually supports, with a comment next to each group
explaining what it found.

```console
$ auto-tlp --report            # 1. see what was detected (read-only)
$ auto-tlp -o my-tlp.conf      # 2. generate a config and review it
$ sudo auto-tlp --install      # 3. install it as /etc/tlp.d/99-auto-tlp.conf
$ sudo tlp start               # 4. apply it
```

Just curious? `auto-tlp --demo` shows what a full-featured laptop's config looks like, on any computer.
After a kernel or TLP update, `auto-tlp --diff` tells you whether the installed config is still what auto-tlp would write.

## Contents

[Install](#install) · [Everyday commands](#everyday-commands) · [How it works](#how-it-works) · [What it detects](#what-it-detects-and-configures) ·
[Profiles](#profiles) · [Example](#example-output) · [Usage](#usage) · [FAQ](#faq) ·
[Building](#building-from-source) · [Packaging status](#packaging-status) · [Contributing](#contributing)

## Install

Pick whatever matches your system. Every release is built for **x86_64 and aarch64**, is statically linked, and
ships with SHA-256 checksums.

| System | Command |
|---|---|
| **Any distro** (one-liner, verifies checksum) | `curl -fsSL https://raw.githubusercontent.com/Dacraezy1/auto-tlp/main/scripts/install.sh \| sh` |
| **Debian / Ubuntu / Mint / Pop!\_OS** | `sudo apt install ./auto-tlp_*_amd64.deb` |
| **Fedora / RHEL / Alma / Rocky** (EPEL for `tlp`) | `sudo dnf install ./auto-tlp-*.rpm` |
| **openSUSE** | `sudo zypper --no-gpg-checks install ./auto-tlp-*.rpm` |
| **Arch / Manjaro / EndeavourOS** | `sudo pacman -U auto-tlp-*.pkg.tar.zst` or build the [`PKGBUILD`](packaging/arch/PKGBUILD) |
| **NixOS / Nix** | `nix run github:Dacraezy1/auto-tlp` |
| **Any distro, no install** | download the `.AppImage`, `chmod +x`, run it |
| **Any distro, manual** | extract the `.tar.gz`, `sudo make install` |
| **Rust users** | `cargo install --git https://github.com/Dacraezy1/auto-tlp` |

Downloads live on the [Releases page](https://github.com/Dacraezy1/auto-tlp/releases/latest).
The distro packages depend on `tlp`, so it gets installed with them.

## Everyday commands

| I want to... | Command |
|---|---|
| see what was detected | `auto-tlp --report` (add `-v` for every PCI and USB device) |
| preview the config | `auto-tlp` |
| generate it for battery life | `auto-tlp -p battery` |
| limit the charge to 60-85% | `auto-tlp --start-thresh 60 --stop-thresh 85` |
| **install** it and apply it right away | `sudo auto-tlp --install --apply` |
| see what **changed** since I installed it | `auto-tlp --diff` (exit status 1 = differences) |
| **remove** it again | `sudo auto-tlp --uninstall --apply` |
| only some settings | `auto-tlp --only cpu,battery` or `auto-tlp --skip usb,radios` |
| use it on **NixOS** | `auto-tlp --format nix > tlp.nix` (a `services.tlp.settings` snippet) |
| feed it to a script | `auto-tlp --format json` |

Sections for `--only` / `--skip`: `general`, `cpu`, `platform`, `graphics`, `storage`, `pcie`, `radios`, `usb`,
`audio`, `kernel`, `battery`.

## How it works

1. **Scan.** auto-tlp reads `/sys`, `/proc` and DMI data. This is **100% read-only**. It never writes
   anything unless you pass `--output` or `--install`.
2. **Decide.** For each detected component it picks only the TLP settings that component supports, and only
   values the kernel actually offers (for example an I/O scheduler that is really available for *that* disk).
3. **Write.** It prints a tidy config, grouped by topic, with the detected facts as comments. Hardware you
   don't have produces no settings at all.

It reads your installed **TLP version** and writes the matching key names (see [TLP versions](#tlp-versions-and-key-names)).

It also warns you about things that would stop TLP from working well: a conflicting daemon
(`power-profiles-daemon`, `tuned`, `auto-cpufreq`...), missing helper tools (`hdparm`, `iw`, `ethtool`), an
old TLP version, and active keys in `/etc/tlp.conf` that would override the generated drop-in.

## What it detects and configures

| Detected | TLP settings produced |
|---|---|
| **CPU**: model, cores, vendor, **scaling driver** (`intel_pstate`, `amd-pstate`, `amd-pstate-epp`, `acpi-cpufreq`), governors, energy/performance preference, turbo switch, HWP dynamic boost | `CPU_SCALING_GOVERNOR_*`, `CPU_ENERGY_PERF_POLICY_*`, `CPU_BOOST_*`, `CPU_HWP_DYN_BOOST_*`, `CPU_MIN/MAX_PERF_*`, `CPU_DRIVER_OPMODE_*`, `CPU_SCALING_MAX_FREQ_ON_BAT` |
| **Firmware platform profile** (the laptop's own performance/thermal mode) | `PLATFORM_PROFILE_ON_*`, using only modes your firmware offers |
| **Battery**: model, health, cycle count, and *which* charge-limit files your kernel driver exposes | `START/STOP_CHARGE_THRESH_BAT0/1` (end-threshold only when start isn't supported) |
| **NVMe / SATA SSD / HDD / eMMC**: type, driver, available I/O schedulers | `DISK_DEVICES`, `DISK_IOSCHED`, `SATA_LINKPWR_*`, `AHCI_RUNTIME_PM_*`, and for HDDs only `DISK_APM_*` and `DISK_SPINDOWN_*` |
| **GPUs**: Intel `i915` frequency range, AMD `amdgpu`/`radeon` DPM, NVIDIA proprietary runtime power state | `INTEL_GPU_*_FREQ_*`, `RADEON_DPM_*`, runtime-PM denylist |
| **PCIe** ASPM policies | `PCIE_ASPM_ON_*`, `RUNTIME_PM_*` |
| **Wi-Fi, Ethernet, Bluetooth, WWAN** | `WIFI_PWR_ON_*`, `WOL_DISABLE`, optional radio switch-off |
| **USB** devices (external mice and keyboards are kept awake) | `USB_AUTOSUSPEND`, `USB_EXCLUDE_*`, `USB_DENYLIST` |
| **Audio** codec power saving | `SOUND_POWER_SAVE_*` |
| **Kernel** extras | `NMI_WATCHDOG`, suspend-mode hint (`MEM_SLEEP_*`, left commented) |

Run `auto-tlp --report --verbose` to see everything it found, including all PCI and USB devices.

## Profiles

Choose with `--profile`. **`balanced`** (the default) follows TLP's own philosophy.

| | `balanced` | `performance` | `battery` |
|---|---|---|---|
| Best for | everyday use | plugged-in power users, gaming | maximum runtime and battery lifespan |
| CPU preference (AC / battery) | balance\_performance / balance\_power | performance / balance\_performance | balance\_performance / power |
| Turbo boost on battery | off | **on** | off |
| CPU speed cap on battery | none | none | 60% |
| Battery charge limit | 75–80% | none (offered commented-out) | 40–80% |
| PCIe power saving on battery | powersave | default | powersupersave |
| Wi-Fi power saving on battery | on | off | on |

Override the charge limit on any profile: `auto-tlp --start-thresh 60 --stop-thresh 85`.

With **TLP 1.11 or newer**, TLP has three runtime profiles of its own, and auto-tlp fills in all three:

| TLP profile | When it is used | Values come from |
|---|---|---|
| `PRF` (performance) | plugged in (TLP's default) | the "AC" tuning of your chosen `--profile` |
| `BAL` (balanced) | on battery (TLP's default) | the "battery" tuning of your chosen `--profile` |
| `SAV` (power-saver) | when you run `tlp power-saver` | always the aggressive `battery` tuning |

## TLP versions and key names

TLP 1.11 renamed its settings: `CPU_BOOST_ON_AC` became `CPU_BOOST_ON_PRF`, `CPU_BOOST_ON_BAT` became `CPU_BOOST_ON_BAL`, and a new `_ON_SAV` power-saver variant appeared.

| Your TLP | What auto-tlp writes | Why |
|---|---|---|
| **1.11 or newer** | `_ON_PRF`, `_ON_BAL`, `_ON_SAV` | the current names; no deprecation hints |
| **older, or not detected** | `_ON_AC`, `_ON_BAT` | understood by every version. New TLP still accepts them but prints *"Runtime-translated ... Change your config!"* |

Generating a config for a machine you are not on? `auto-tlp --tlp-version 1.11`.
Settings TLP removed or deprecated (`RADEON_DPM_STATE_*`, `TLP_PERSISTENT_DEFAULT`) are never written for new TLP.

## Example output

Preview it yourself with `auto-tlp --demo`. This is the built-in sample laptop (hybrid-graphics ThinkPad with
NVMe + SATA SSD + HDD) for TLP 1.11; the full files are
[`docs/example-config.conf`](docs/example-config.conf) and, for older TLP,
[`docs/example-config-classic.conf`](docs/example-config-classic.conf).

```ini
# ===== CPU =====
# 11th Gen Intel(R) Core(TM) i7-1185G7 - 4 cores / 8 threads
# scaling driver: intel_pstate (active mode); governors: performance powersave
CPU_SCALING_GOVERNOR_ON_PRF="powersave"
CPU_SCALING_GOVERNOR_ON_BAL="powersave"
CPU_SCALING_GOVERNOR_ON_SAV="powersave"
CPU_ENERGY_PERF_POLICY_ON_PRF="balance_performance"
CPU_ENERGY_PERF_POLICY_ON_BAL="balance_power"
CPU_ENERGY_PERF_POLICY_ON_SAV="power"
CPU_BOOST_ON_PRF="1"
CPU_BOOST_ON_BAL="0"
CPU_BOOST_ON_SAV="0"

# ===== Storage =====
# nvme0n1: NVMe SSD Samsung SSD 980 PRO 512GB [NVMe, driver nvme, scheduler none]
# sdb: HDD WDC WD10SPZX-24Z10 [SATA, driver ahci, scheduler mq-deadline]
DISK_DEVICES="nvme0n1 sda sdb"
DISK_IOSCHED="none mq-deadline bfq"
DISK_APM_LEVEL_ON_BAL="keep keep 128"      # only the HDD gets APM settings

# ===== Battery care =====
# BAT0: L17M3P55 (Li-poly) health 91%, 212 cycles
START_CHARGE_THRESH_BAT0="75"
STOP_CHARGE_THRESH_BAT0="80"
```

## Usage

```text
auto-tlp [OPTIONS]

  -p, --profile <NAME>     balanced (default) | performance | battery
      --start-thresh <N>   battery start-charge threshold (0-99)
      --stop-thresh <N>    battery stop-charge threshold (1-100)
  -o, --output <FILE>      write the config to FILE instead of stdout
      --install            write /etc/tlp.d/99-auto-tlp.conf (needs root; backs up an existing file)
  -r, --report             only print the detected hardware report
  -v, --verbose            with --report: also list all PCI/USB devices and sensors
      --only <LIST>        only these sections (comma separated)
      --skip <LIST>        leave these sections out
      --terse              omit explanatory comments
      --format <FMT>       conf (default) | nix | json
      --tlp-version <X.Y>  generate for this TLP version instead of the installed one

  -o, --output <FILE>      write to FILE instead of stdout ('-' = stdout)
      --install            write /etc/tlp.d/99-auto-tlp.conf (needs root; backs up an existing file)
      --uninstall          remove that file (needs root)
      --apply              with --install/--uninstall: run `tlp start` afterwards
      --diff               compare with the installed config; exit status 1 if different
      --against <FILE>     with --diff: compare with FILE instead

  -r, --report             only print the detected hardware report
  -v, --verbose            with --report: also list all PCI/USB devices and sensors
      --demo               use a built-in sample laptop instead of scanning this machine
      --sysroot <DIR>      scan a directory tree instead of the live system (for tests)
  -h, --help / -V, --version
```

Shell completions (bash, zsh, fish) and a man page (`man auto-tlp`) are installed by the packages.

## FAQ

**Is it safe?** Scanning only reads files. Nothing changes on your system unless you run `--install` (which writes a
single drop-in file and keeps a `.bak` of any previous one). Read the generated config first; it is plain text.
To undo: `sudo rm /etc/tlp.d/99-auto-tlp.conf && sudo tlp start`.

**Why `/etc/tlp.d/` and not `/etc/tlp.conf`?** Drop-ins keep your own edits separate and survive TLP upgrades.
Settings that are *active* in `/etc/tlp.conf` still win, and auto-tlp warns you when that happens.

**"No charge threshold support" on my laptop.** Charge limits need a kernel driver that exposes them
(ThinkPad, ASUS, Lenovo IdeaPad, Framework, Dell, and several others do). If yours doesn't,
auto-tlp says so and writes no threshold keys instead of guessing.

**My distro already runs `power-profiles-daemon` (GNOME/KDE).** It conflicts with TLP. auto-tlp tells you,
and the fix is `sudo systemctl disable --now power-profiles-daemon`.

**Hybrid NVIDIA laptop.** If the proprietary driver's dynamic power management is off, auto-tlp excludes it
from runtime PM so TLP doesn't fight it, and warns you how to enable
`NVreg_DynamicPowerManagement=0x02` if you want the GPU to power off.

**Which TLP versions work?** All of them. auto-tlp reads your installed version and writes the matching key
names; see [TLP versions and key names](#tlp-versions-and-key-names).

**What changed after I updated my kernel or TLP?** Run `auto-tlp --diff`. It lists settings that are new (`+`),
gone (`-`) or different (`~`) compared with the installed config, and exits with status 1 if anything differs.

**A device was misdetected.** Please [open an issue](https://github.com/Dacraezy1/auto-tlp/issues/new/choose)
with the output of `auto-tlp --report --verbose`. These reports are the most valuable contribution.

## Building from source

```console
$ git clone https://github.com/Dacraezy1/auto-tlp && cd auto-tlp
$ cargo build --release          # needs Rust 1.75+, no other dependencies
$ cargo test                     # unit + end-to-end tests
$ sudo make install PREFIX=/usr/local
```

There are **no crate dependencies**, so the build works offline and the whole program is easy to audit.

```text
src/hw.rs        detection (sysfs/procfs/DMI) -> plain data structs
src/gen.rs       data structs -> TLP config (profiles, rules, warnings) + unit tests
src/report.rs    human-readable --report
src/demo.rs      built-in sample laptop for --demo, CI and tests
src/main.rs      CLI
tests/cli.rs     end-to-end tests of the real binary
packaging/       recipes for Arch, Alpine, Void, Gentoo, RPM, AppImage
nix/, flake.nix  Nix package
scripts/         release, install and test helpers
.github/         CI + release pipeline
```

## Packaging status

Every push runs this matrix in GitHub Actions. Maintainers: see [`docs/RELEASING.md`](docs/RELEASING.md).

| Format | Built and tested in CI |
|---|---|
| Hardware detection | ✅ the real binary is run against fake machines (Intel + hybrid NVIDIA laptop, AMD desktop, NVMe/SATA/HDD, batteries, odd or missing files) |
| Static `.tar.gz` (x86\_64, aarch64) | ✅ runs on glibc and musl (Alpine) |
| `.deb` | ✅ installed in clean Debian and Ubuntu containers |
| `.rpm` | ✅ installed in Fedora, openSUSE and AlmaLinux containers |
| `.pkg.tar.zst` (Arch) | ✅ built with `makepkg` from the `PKGBUILD`, then installed |
| `.AppImage` | ✅ executed |
| Nix flake | ✅ `nix build` |
| Generated keys vs **real TLP** | ✅ every key checked against TLP 1.6 (from apt) and against upstream TLP built from source, which must report no deprecated or renamed settings |
| Source build with each distro's own Rust | ✅ Ubuntu, Fedora, Alpine, openSUSE, Arch |
| Alpine `APKBUILD`, Void `template`, Gentoo `ebuild`, source `.spec` | 📝 provided as starting points, not CI-tested; PRs welcome |

## Contributing

Bug reports with a hardware report, new device support and tests are all welcome. See
[CONTRIBUTING.md](CONTRIBUTING.md). The one rule: **a TLP key must only be emitted when the hardware supports it.**

## License

Copyright © 2026 Dacraezy1. Licensed under the **GNU General Public License v3.0 or later**; see [LICENSE](LICENSE).
