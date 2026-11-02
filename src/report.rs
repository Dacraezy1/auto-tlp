// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dacraezy1
//! Human readable hardware report (`--report`).
use crate::hw::*;
use crate::util::*;

fn h(t: &str) {
    println!("\n== {} ==", t);
}

pub fn print(hw: &Hardware, verbose: bool) {
    let p = &hw.platform;
    h("Platform");
    println!("Machine : {} {} {}", p.vendor, p.product, p.version);
    println!(
        "Chassis : {}{}",
        p.chassis,
        if hw.is_laptop() {
            " (treated as laptop)"
        } else {
            ""
        }
    );
    println!("BIOS    : {}", p.bios);
    println!("OS      : {} / kernel {}", p.distro, p.kernel);
    if let Some(kb) = p.mem_total_kb {
        println!("Memory  : {}", human_bytes(kb * 1024));
    }
    if !p.vendor_modules.is_empty() {
        println!("Vendor platform drivers: {}", p.vendor_modules.join(", "));
    }
    if let Some((c, cur)) = &p.pcie_aspm {
        println!(
            "PCIe ASPM: {} [current: {}]",
            c.join(" "),
            cur.as_deref().unwrap_or("?")
        );
    }
    if let Some((c, cur)) = &p.mem_sleep {
        println!(
            "Suspend  : {} [current: {}]",
            c.join(" "),
            cur.as_deref().unwrap_or("?")
        );
    }

    let c = &hw.cpu;
    h("CPU");
    println!("Model   : {}", c.model);
    println!("Topology: {} cores / {} threads", c.physical, c.logical);
    println!(
        "Driver  : {}{}",
        c.driver.as_deref().unwrap_or("unknown (no cpufreq)"),
        c.intel_pstate
            .as_ref()
            .or(c.amd_pstate.as_ref())
            .map(|m| format!(" [{}]", m))
            .unwrap_or_default()
    );
    println!(
        "Governor: {} (available: {})",
        c.governor.as_deref().unwrap_or("?"),
        c.governors.join(" ")
    );
    if !c.epp_choices.is_empty() {
        println!(
            "EPP     : {} (available: {})",
            c.epp.as_deref().unwrap_or("?"),
            c.epp_choices.join(" ")
        );
    } else if c.epb {
        println!("EPB     : legacy energy_perf_bias supported");
    }
    if let (Some(a), Some(b)) = (c.min_khz, c.max_khz) {
        println!("Freq    : {} - {} MHz", a / 1000, b / 1000);
    }
    println!(
        "Boost   : {}",
        c.boost_knob.as_deref().unwrap_or("no control found")
    );
    if c.hwp {
        println!(
            "HWP     : yes{}",
            if c.hwp_dyn_boost {
                " (dynamic boost available)"
            } else {
                ""
            }
        );
    }
    if !c.platform_profiles.is_empty() {
        println!(
            "Platform profile: {} (available: {})",
            c.platform_profile.as_deref().unwrap_or("?"),
            c.platform_profiles.join(" ")
        );
    }

    h("Battery / power");
    if hw.batteries.is_empty() {
        println!("No battery found.");
    }
    for b in &hw.batteries {
        println!(
            "{}: {} {} [{}] status={}",
            b.name,
            b.manufacturer.as_deref().unwrap_or(""),
            b.model.as_deref().unwrap_or(""),
            b.technology.as_deref().unwrap_or("?"),
            b.status.as_deref().unwrap_or("?")
        );
        if let (Some(d), Some(f)) = (b.design, b.full) {
            println!(
                "   design {:.1} {} -> full {:.1} {}{}",
                d as f64 / 1e6,
                b.unit,
                f as f64 / 1e6,
                b.unit,
                b.health
                    .map(|x| format!(" ({:.0}% health)", x))
                    .unwrap_or_default()
            );
        }
        if let Some(c) = b.cycles {
            println!("   cycles: {}", c);
        }
        println!(
            "   charge thresholds: start={} stop={}{}",
            if b.has_start {
                b.cur_start
                    .map(|v| v.to_string())
                    .unwrap_or("supported".into())
            } else {
                "unsupported".into()
            },
            if b.has_stop {
                b.cur_stop
                    .map(|v| v.to_string())
                    .unwrap_or("supported".into())
            } else {
                "unsupported".into()
            },
            if b.has_behaviour {
                ", charge_behaviour available"
            } else {
                ""
            }
        );
    }
    for a in &hw.ac {
        println!(
            "{} ({}): {}",
            a.name,
            a.kind,
            if a.online { "online" } else { "offline" }
        );
    }

    h("Storage");
    for d in &hw.disks {
        println!(
            "{:<10} {:<9} {:<8} {:>9}  {}  driver={} sched={}{}",
            d.name,
            d.kind(),
            d.transport.label(),
            human_bytes(d.size_bytes),
            d.model,
            d.driver.as_deref().unwrap_or("?"),
            d.sched.as_deref().unwrap_or("?"),
            if d.removable { " (removable)" } else { "" }
        );
    }
    if let Some(s) = &hw.sata_link_pm {
        println!("SATA link power policy: {}", s);
    }
    if let Some(l) = p.nvme_apst_latency_us {
        println!("NVMe APST max latency param: {} us", l);
    }

    h("Graphics");
    if hw.gpus.is_empty() {
        println!("No PCI GPU found.");
    }
    for g in &hw.gpus {
        println!(
            "{} - {} [{}] driver={} card={}",
            g.addr,
            g.name,
            g.role,
            g.driver.as_deref().unwrap_or("none"),
            g.card.as_deref().unwrap_or("?")
        );
        if let Some(r) = &g.runtime_status {
            println!("    runtime PM status: {}", r);
        }
        if let Some(l) = &g.dpm_level {
            println!("    DPM level: {}", l);
        }
        if let Some((n, p1, p0)) = g.intel_freqs {
            println!(
                "    freq range: {} / {} / {} MHz (RPn/RP1/RP0)",
                n,
                p1.map(|v| v.to_string()).unwrap_or("-".into()),
                p0
            );
        }
        if let Some(n) = g.nvidia_dyn_pm {
            println!("    NVIDIA DynamicPowerManagement: {}", n);
        }
    }

    h("Network / radios / audio");
    for n in &hw.net {
        println!(
            "{:<10} {:?} driver={} bus={}",
            n.name,
            n.kind,
            n.driver.as_deref().unwrap_or("?"),
            n.bus
        );
    }
    for b in &hw.bluetooth {
        println!(
            "{} (bluetooth) driver={}",
            b.name,
            b.driver.as_deref().unwrap_or("?")
        );
    }
    if !hw.audio.cards.is_empty() {
        println!(
            "Audio cards: {} (hda_intel pm: {}, ac97 pm: {})",
            hw.audio.cards.join(", "),
            hw.audio.hda,
            hw.audio.ac97
        );
    }

    h("Power management software");
    println!(
        "TLP: {}",
        if hw.tlp.installed {
            hw.tlp
                .version_str
                .clone()
                .unwrap_or("installed (version unknown)".into())
        } else {
            "not installed".into()
        }
    );
    println!(
        "Helpers: hdparm={} iw={} ethtool={}",
        hw.tlp.has_hdparm, hw.tlp.has_iw, hw.tlp.has_ethtool
    );
    for s in &hw.services {
        println!(
            "{:<24} active={} enabled={}{}",
            s.name,
            s.active,
            s.enabled,
            if s.conflicts_with_tlp {
                "  <-- conflicts with TLP"
            } else {
                ""
            }
        );
    }

    if verbose {
        h("USB devices");
        for u in &hw.usb {
            println!(
                "{}  {}  hid={} removable={}",
                u.id, u.name, u.hid, u.removable
            );
        }
        h("PCI devices");
        for d in &hw.pci {
            println!(
                "{}  {:<22} {} {} [{}] driver={}",
                d.addr,
                d.class,
                d.vendor_name,
                d.device_name,
                d.ids,
                d.driver.as_deref().unwrap_or("-")
            );
        }
        h("Sensors");
        println!("hwmon: {}", p.hwmon.join(", "));
        println!("thermal zones: {}", p.thermal_zones.join(", "));
        println!("cmdline: {}", p.cmdline);
    }
}
