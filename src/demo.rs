// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dacraezy1
//! A built-in sample machine: a hybrid-graphics ThinkPad with NVMe + SATA SSD + HDD.
//!
//! Used by `auto-tlp --demo` (preview the full range of output on any computer, validate the generated
//! keys against a TLP install in CI) and by the unit tests.
use crate::hw::*;

pub fn battery(name: &str, start: bool, stop: bool) -> Battery {
    Battery {
        name: name.into(),
        manufacturer: Some("SMP".into()),
        model: Some("L17M3P55".into()),
        technology: Some("Li-poly".into()),
        status: Some("Discharging".into()),
        capacity: Some(80),
        design: Some(45_000_000),
        full: Some(41_000_000),
        unit: "Wh",
        cycles: Some(212),
        health: Some(91.1),
        has_start: start,
        has_stop: stop,
        cur_start: Some(0),
        cur_stop: Some(100),
        has_behaviour: false,
    }
}

pub fn disk(name: &str, t: Transport, rot: bool, scheds: &[&str]) -> Disk {
    Disk {
        name: name.into(),
        model: match (t, rot) {
            (Transport::Nvme, _) => "Samsung SSD 980 PRO 512GB",
            (_, true) => "WDC WD10SPZX-24Z10",
            _ => "Samsung SSD 870 EVO 500GB",
        }
        .into(),
        firmware: None,
        size_bytes: match (t, rot) {
            (Transport::Nvme, _) => 512_110_190_592,
            (_, true) => 1_000_204_886_016,
            _ => 500_107_862_016,
        },
        rotational: rot,
        removable: false,
        transport: t,
        scheds: scheds.iter().map(|s| s.to_string()).collect(),
        sched: scheds.first().map(|s| s.to_string()),
        driver: Some(if t == Transport::Nvme { "nvme" } else { "ahci" }.into()),
    }
}

pub fn strs(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

pub fn intel_gpu() -> Gpu {
    Gpu {
        addr: "0000:00:02.0".into(),
        vendor: GpuVendor::Intel,
        name: "Intel Corporation TigerLake-LP GT2".into(),
        driver: Some("i915".into()),
        role: "integrated",
        card: Some("card0".into()),
        dpm_level: None,
        has_dpm_state: false,
        runtime_status: Some("active".into()),
        intel_freqs: Some((100, Some(700), 1150)),
        nvidia_dyn_pm: None,
    }
}

pub fn nvidia_gpu() -> Gpu {
    Gpu {
        addr: "0000:01:00.0".into(),
        vendor: GpuVendor::Nvidia,
        name: "NVIDIA Corporation GA106M".into(),
        driver: Some("nvidia".into()),
        role: "discrete",
        card: Some("card1".into()),
        dpm_level: None,
        has_dpm_state: false,
        runtime_status: Some("active".into()),
        intel_freqs: None,
        nvidia_dyn_pm: Some(0),
    }
}

/// A realistic hybrid-graphics Intel laptop with NVMe + SATA SSD + HDD.
pub fn thinkpad() -> Hardware {
    Hardware {
        platform: Platform {
            vendor: "LENOVO".into(),
            product: "ThinkPad X1 Carbon".into(),
            chassis: "Notebook".into(),
            chassis_is_portable: true,
            pcie_aspm: Some((
                strs(&["default", "performance", "powersave", "powersupersave"]),
                Some("default".into()),
            )),
            mem_sleep: Some((strs(&["s2idle", "deep"]), Some("s2idle".into()))),
            nmi_watchdog: true,
            ..Default::default()
        },
        cpu: Cpu {
            model: "11th Gen Intel(R) Core(TM) i7-1185G7".into(),
            vendor: CpuVendor::Intel,
            logical: 8,
            physical: 4,
            driver: Some("intel_pstate".into()),
            governors: strs(&["performance", "powersave"]),
            governor: Some("powersave".into()),
            epp_choices: strs(&[
                "default",
                "performance",
                "balance_performance",
                "balance_power",
                "power",
            ]),
            epp: Some("balance_performance".into()),
            min_khz: Some(400_000),
            max_khz: Some(4_800_000),
            boost_knob: Some("intel_pstate/no_turbo".into()),
            intel_pstate: Some("active".into()),
            intel_perf_pct: true,
            hwp: true,
            hwp_dyn_boost: true,
            platform_profiles: strs(&["low-power", "balanced", "performance"]),
            platform_profile: Some("balanced".into()),
            ..Default::default()
        },
        batteries: vec![battery("BAT0", true, true)],
        ac: vec![],
        disks: vec![
            disk("nvme0n1", Transport::Nvme, false, &["none", "mq-deadline"]),
            disk("sda", Transport::Sata, false, &["mq-deadline", "none"]),
            disk(
                "sdb",
                Transport::Sata,
                true,
                &["mq-deadline", "bfq", "none"],
            ),
        ],
        sata_link_pm: Some("max_performance".into()),
        gpus: vec![intel_gpu(), nvidia_gpu()],
        net: vec![
            NetIf {
                name: "wlp0s20f3".into(),
                kind: NetKind::Wifi,
                driver: Some("iwlwifi".into()),
                bus: "pci/other",
            },
            NetIf {
                name: "enp0s31f6".into(),
                kind: NetKind::Ethernet,
                driver: Some("e1000e".into()),
                bus: "pci/other",
            },
        ],
        bluetooth: vec![Bluetooth {
            name: "hci0".into(),
            driver: Some("btusb".into()),
        }],
        audio: Audio {
            hda: true,
            ac97: false,
            cards: strs(&["PCH"]),
        },
        usb: vec![
            UsbDev {
                id: "046d:c52b".into(),
                name: "Logitech Receiver".into(),
                hid: true,
                bluetooth: false,
                removable: "removable".into(),
            },
            UsbDev {
                id: "04f2:b6d9".into(),
                name: "Integrated Camera".into(),
                hid: false,
                bluetooth: false,
                removable: "fixed".into(),
            },
            UsbDev {
                id: "8087:0033".into(),
                name: "Intel Bluetooth".into(),
                hid: false,
                bluetooth: true,
                removable: "fixed".into(),
            },
        ],
        services: vec![],
        tlp: TlpInfo {
            installed: true,
            version: Some((1, 7)),
            version_str: Some("1.7.0".into()),
            has_hdparm: true,
            has_iw: true,
            has_ethtool: true,
            has_dropin_dir: true,
        },
        pci: vec![],
    }
}
