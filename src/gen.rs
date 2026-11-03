// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dacraezy1
//! Turns detected hardware into a TLP configuration.
use crate::hw::*;
use crate::util::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Profile {
    Balanced,
    Performance,
    Battery,
}

impl Profile {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "balanced" | "default" => Some(Profile::Balanced),
            "performance" | "perf" => Some(Profile::Performance),
            "battery" | "saver" | "powersave" | "longevity" => Some(Profile::Battery),
            _ => None,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Profile::Balanced => "balanced",
            Profile::Performance => "performance",
            Profile::Battery => "battery",
        }
    }
    fn vals(self) -> PV {
        match self {
            Profile::Balanced => PV {
                epp_ac: "balance_performance",
                epp_bat: "balance_power",
                boost_bat: false,
                max_perf_bat: 100,
                thresh: Some((75, 80)),
                pp_ac: &["performance", "balanced-performance", "balanced"],
                pp_bat: &["low-power", "quiet", "cool", "balanced"],
                aspm_bat: "powersave",
                sata_bat: "med_power_with_dipm",
                wifi_bat: "on",
                hdd_apm_bat: 128,
                hdd_spin_bat: 120,
                gov_ac_perf: false,
            },
            Profile::Performance => PV {
                epp_ac: "performance",
                epp_bat: "balance_performance",
                boost_bat: true,
                max_perf_bat: 100,
                thresh: None,
                pp_ac: &["performance"],
                pp_bat: &["balanced", "balanced-performance"],
                aspm_bat: "default",
                sata_bat: "max_performance",
                wifi_bat: "off",
                hdd_apm_bat: 254,
                hdd_spin_bat: 0,
                gov_ac_perf: true,
            },
            Profile::Battery => PV {
                epp_ac: "balance_performance",
                epp_bat: "power",
                boost_bat: false,
                max_perf_bat: 60,
                thresh: Some((40, 80)),
                pp_ac: &["balanced", "balanced-performance", "performance"],
                pp_bat: &["low-power", "quiet", "cool", "balanced"],
                aspm_bat: "powersupersave",
                sata_bat: "med_power_with_dipm",
                wifi_bat: "on",
                hdd_apm_bat: 128,
                hdd_spin_bat: 60,
                gov_ac_perf: false,
            },
        }
    }
}

#[derive(Clone, Copy)]
struct PV {
    epp_ac: &'static str,
    epp_bat: &'static str,
    boost_bat: bool,
    max_perf_bat: u8,
    thresh: Option<(u8, u8)>,
    pp_ac: &'static [&'static str],
    pp_bat: &'static [&'static str],
    aspm_bat: &'static str,
    sata_bat: &'static str,
    wifi_bat: &'static str,
    hdd_apm_bat: u8,
    hdd_spin_bat: u16,
    gov_ac_perf: bool,
}

pub struct Options {
    pub profile: Profile,
    pub start: Option<u8>,
    pub stop: Option<u8>,
    pub terse: bool,
}

pub struct Generated {
    pub text: String,
    pub warnings: Vec<String>,
}

/// Key naming scheme. TLP 1.11 renamed every `*_ON_AC` / `*_ON_BAT` setting to `*_ON_PRF` / `*_ON_BAL`
/// and added a power-saver profile (`*_ON_SAV`). Old names keep working in new TLP (converted at runtime),
/// but new names do NOT work in old TLP, so the legacy names are used unless TLP >= 1.11 is known.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Scheme {
    Legacy,
    Profiles,
}

/// Settings that have a power-saver (`_ON_SAV`) variant in TLP 1.11 (taken from upstream's tlp.conf).
const SAV_KEYS: &[&str] = &[
    "CPU_DRIVER_OPMODE",
    "CPU_SCALING_GOVERNOR",
    "CPU_SCALING_MIN_FREQ",
    "CPU_SCALING_MAX_FREQ",
    "CPU_ENERGY_PERF_POLICY",
    "CPU_MIN_PERF",
    "CPU_MAX_PERF",
    "CPU_BOOST",
    "CPU_HWP_DYN_BOOST",
    "PLATFORM_PROFILE",
    "MEM_SLEEP",
    "DISK_APM_LEVEL",
    "DISK_SPINDOWN_TIMEOUT",
    "SATA_LINKPWR",
    "AHCI_RUNTIME_PM",
    "INTEL_GPU_POWER_PROFILE",
    "INTEL_GPU_MIN_FREQ",
    "INTEL_GPU_MAX_FREQ",
    "INTEL_GPU_BOOST_FREQ",
    "RADEON_DPM_PERF_LEVEL",
    "AMDGPU_ABM_LEVEL",
    "WIFI_PWR",
    "SOUND_POWER_SAVE",
    "PCIE_ASPM",
    "RUNTIME_PM",
];

struct W {
    s: String,
    terse: bool,
    scheme: Scheme,
    /// Values for TLP's power-saver profile (always the aggressive "battery" tuning).
    sv: PV,
}

impl W {
    fn raw(&mut self, l: &str) {
        self.s.push_str(l);
        self.s.push('\n');
    }
    fn section(&mut self, t: &str) {
        self.s.push('\n');
        self.raw(&format!("# ===== {} =====", t));
    }
    /// Detected facts and explanations (dropped with --terse).
    fn info(&mut self, t: &str) {
        if self.terse {
            return;
        }
        for l in t.lines() {
            self.raw(&format!("# {}", l));
        }
    }
    fn kv(&mut self, k: &str, v: &str) {
        self.raw(&format!("{}=\"{}\"", k, v));
    }
    fn ckv(&mut self, k: &str, v: &str) {
        self.raw(&format!("#{}=\"{}\"", k, v));
    }

    fn put(&mut self, commented: bool, k: &str, v: &str) {
        if commented {
            self.ckv(k, v)
        } else {
            self.kv(k, v)
        }
    }

    /// Emit a setting that differs by power source: `ac` = plugged in, `bat` = on battery,
    /// `sav` = TLP's power-saver profile (only written for TLP >= 1.11, and only for keys that have one).
    fn dual_impl(&mut self, commented: bool, base: &str, ac: Option<&str>, bat: &str, sav: &str) {
        match self.scheme {
            Scheme::Legacy => {
                if let Some(a) = ac {
                    self.put(commented, &format!("{base}_ON_AC"), a);
                }
                self.put(commented, &format!("{base}_ON_BAT"), bat);
            }
            Scheme::Profiles => {
                if let Some(a) = ac {
                    self.put(commented, &format!("{base}_ON_PRF"), a);
                }
                self.put(commented, &format!("{base}_ON_BAL"), bat);
                if SAV_KEYS.contains(&base) {
                    self.put(commented, &format!("{base}_ON_SAV"), sav);
                }
            }
        }
    }
    fn dual(&mut self, base: &str, ac: &str, bat: &str, sav: &str) {
        self.dual_impl(false, base, Some(ac), bat, sav);
    }
    fn dual_c(&mut self, base: &str, ac: &str, bat: &str, sav: &str) {
        self.dual_impl(true, base, Some(ac), bat, sav);
    }
}

fn flag(on: bool) -> &'static str {
    if on {
        "1"
    } else {
        "0"
    }
}

fn pick(choices: &[String], prefs: &[&str]) -> Option<String> {
    prefs
        .iter()
        .find(|p| choices.iter().any(|c| c == **p))
        .map(|s| s.to_string())
}

pub fn generate(hw: &Hardware, o: &Options) -> Generated {
    let pv = o.profile.vals();
    let mut w = W {
        s: String::new(),
        terse: o.terse,
        scheme: if hw.tlp.uses_profiles() {
            Scheme::Profiles
        } else {
            Scheme::Legacy
        },
        sv: Profile::Battery.vals(),
    };
    let mut warn: Vec<String> = Vec::new();

    header(&mut w, hw, o);
    general(&mut w, hw, &mut warn);
    cpu(&mut w, hw, &pv, &mut warn);
    platform_profile(&mut w, hw, &pv);
    graphics(&mut w, hw, o.profile, &mut warn);
    storage(&mut w, hw, &pv, &mut warn);
    pcie(&mut w, hw, &pv, &mut warn);
    radios(&mut w, hw, &pv, &mut warn);
    usb(&mut w, hw);
    audio(&mut w, hw);
    misc(&mut w, hw);
    battery(&mut w, hw, &pv, o, &mut warn);
    environment_warnings(hw, &mut warn);

    Generated {
        text: w.s,
        warnings: warn,
    }
}

// ───────────────────────────── sections ─────────────────────────────

fn header(w: &mut W, hw: &Hardware, o: &Options) {
    let date = run("date", &["-u", "+%Y-%m-%d %H:%M UTC"]).unwrap_or_else(|| "unknown date".into());
    let p = &hw.platform;
    w.raw("# TLP configuration generated by auto-tlp");
    w.raw(&format!("# Generated : {}", date));
    w.raw(&format!("# Profile   : {}", o.profile.name()));
    w.raw(&format!(
        "# Machine   : {} {} ({})",
        p.vendor, p.product, p.chassis
    ));
    w.raw(&format!("# CPU       : {}", hw.cpu.model));
    w.raw(&match (&hw.tlp.version_str, hw.tlp.uses_profiles()) {
        (Some(v), true) => format!("# TLP       : {v} (profile-style keys: _ON_PRF / _ON_BAL / _ON_SAV)"),
        (Some(v), false) => format!("# TLP       : {v} (classic keys: _ON_AC / _ON_BAT)"),
        (None, _) => "# TLP       : version unknown, using classic _ON_AC / _ON_BAT keys (understood by every TLP version)".to_string(),
    });
    for g in &hw.gpus {
        w.raw(&format!(
            "# GPU       : {} [{}, driver: {}]",
            g.name,
            g.role,
            g.driver.as_deref().unwrap_or("none")
        ));
    }
    for d in hw
        .disks
        .iter()
        .filter(|d| !d.removable && d.transport != Transport::Usb)
    {
        w.raw(&format!(
            "# Disk      : {} {} {} ({})",
            d.name,
            d.kind(),
            d.model,
            human_bytes(d.size_bytes)
        ));
    }
    w.raw("# Install as /etc/tlp.d/99-auto-tlp.conf, then run: sudo tlp start");
}

fn general(w: &mut W, hw: &Hardware, warn: &mut Vec<String>) {
    w.section("General");
    w.kv("TLP_ENABLE", "1");
    if !hw.is_laptop() {
        w.info("No battery detected: pin TLP to the AC/performance profile permanently.");
        match w.scheme {
            Scheme::Legacy => {
                w.kv("TLP_DEFAULT_MODE", "AC");
                w.kv("TLP_PERSISTENT_DEFAULT", "1");
            }
            Scheme::Profiles => {
                w.kv("TLP_AUTO_SWITCH", "0");
                w.kv("TLP_PROFILE_DEFAULT", "PRF");
            }
        }
        warn.push("No battery found (desktop?). TLP is pinned to AC mode; most battery tuning is irrelevant here.".into());
    }
}

fn cpu(w: &mut W, hw: &Hardware, pv: &PV, o_warn: &mut Vec<String>) {
    let c = &hw.cpu;
    w.section("CPU");
    let drv = c.driver.as_deref().unwrap_or("unknown");
    let mode = c
        .intel_pstate
        .as_ref()
        .or(c.amd_pstate.as_ref())
        .map(|m| format!(" ({} mode)", m))
        .unwrap_or_default();
    w.info(&format!(
        "{} - {} cores / {} threads",
        c.model, c.physical, c.logical
    ));
    w.info(&format!(
        "scaling driver: {}{}; governors: {}",
        drv,
        mode,
        c.governors.join(" ")
    ));

    if let Some(st) = &c.amd_pstate {
        if hw.tlp.at_least(1, 5) && ["active", "guided", "passive"].contains(&st.as_str()) {
            w.info("amd-pstate operation mode is kept as currently running.");
            w.dual("CPU_DRIVER_OPMODE", st, st, st);
        }
    }

    // Governor
    let has_epp = !c.epp_choices.is_empty();
    let dynamic_gov = c
        .governors
        .iter()
        .any(|g| ["schedutil", "ondemand", "conservative"].contains(&g.as_str()));
    if !c.governors.is_empty() {
        let (ac, bat) = if !dynamic_gov {
            // intel_pstate / amd-pstate-epp "active" mode: powersave is the efficient governor,
            // the energy/performance preference does the actual tuning.
            (
                pick(&c.governors, &["powersave"]),
                pick(&c.governors, &["powersave"]),
            )
        } else if pv.gov_ac_perf {
            (
                pick(&c.governors, &["performance", "schedutil"]),
                pick(&c.governors, &["schedutil", "ondemand"]),
            )
        } else {
            (
                pick(&c.governors, &["schedutil", "ondemand", "performance"]),
                pick(
                    &c.governors,
                    &["schedutil", "conservative", "ondemand", "powersave"],
                ),
            )
        };
        let sav = if dynamic_gov {
            pick(
                &c.governors,
                &["schedutil", "conservative", "ondemand", "powersave"],
            )
        } else {
            pick(&c.governors, &["powersave"])
        };
        if let (Some(a), Some(b)) = (ac, bat) {
            w.dual("CPU_SCALING_GOVERNOR", &a, &b, sav.as_deref().unwrap_or(&b));
        }
    }

    // Energy/performance preference (HWP.EPP, amd-pstate EPP or legacy EPB)
    if has_epp || c.epb {
        let valid = |v: &str| !has_epp || c.epp_choices.iter().any(|x| x == v);
        let ac = if valid(pv.epp_ac) {
            pv.epp_ac
        } else {
            "default"
        };
        let bat = if valid(pv.epp_bat) {
            pv.epp_bat
        } else {
            "default"
        };
        let sav = if valid(w.sv.epp_bat) {
            w.sv.epp_bat
        } else {
            "default"
        };
        w.info(if has_epp {
            "Energy/performance preference (EPP) is supported by the driver."
        } else {
            "No EPP; using the legacy energy_perf_bias (EPB) interface."
        });
        w.dual("CPU_ENERGY_PERF_POLICY", ac, bat, sav);
    }

    // Turbo / boost
    if let Some(k) = &c.boost_knob {
        w.info(&format!("Turbo/boost control found ({}).", k));
        w.dual("CPU_BOOST", "1", flag(pv.boost_bat), flag(w.sv.boost_bat));
    }
    if c.hwp_dyn_boost {
        w.dual(
            "CPU_HWP_DYN_BOOST",
            "1",
            flag(pv.boost_bat),
            flag(w.sv.boost_bat),
        );
    }

    // Performance caps
    if c.intel_perf_pct {
        w.dual("CPU_MIN_PERF", "0", "0", "0");
        w.dual(
            "CPU_MAX_PERF",
            "100",
            &pv.max_perf_bat.to_string(),
            &w.sv.max_perf_bat.to_string(),
        );
    } else if let Some(max) = c.max_khz {
        // No intel_pstate percentage knob: cap the maximum frequency (kHz) instead.
        let cap = |pct: u8| ((max * pct as u64 / 100) / 1000 * 1000).to_string();
        let sav_pct = w.sv.max_perf_bat;
        match w.scheme {
            Scheme::Legacy if pv.max_perf_bat < 100 => {
                w.info("No intel_pstate percentage knob; capping max frequency (kHz) on battery instead.");
                w.kv("CPU_SCALING_MAX_FREQ_ON_BAT", &cap(pv.max_perf_bat));
            }
            Scheme::Profiles => {
                if pv.max_perf_bat < 100 {
                    w.kv("CPU_SCALING_MAX_FREQ_ON_BAL", &cap(pv.max_perf_bat));
                }
                if sav_pct < 100 {
                    w.info("No intel_pstate percentage knob; capping max frequency (kHz) in the power-saver profile.");
                    w.kv("CPU_SCALING_MAX_FREQ_ON_SAV", &cap(sav_pct));
                }
            }
            Scheme::Legacy => {}
        }
    }

    // Driver advice
    if drv == "acpi-cpufreq" {
        match c.vendor {
            CpuVendor::Intel => o_warn.push("CPU uses acpi-cpufreq. Modern Intel CPUs do better with intel_pstate (remove intel_pstate=disable from the kernel cmdline).".into()),
            CpuVendor::Amd => o_warn.push("CPU uses acpi-cpufreq. On Zen 2+ with a recent kernel, amd_pstate (kernel param amd_pstate=active) gives much better efficiency.".into()),
            _ => {}
        }
    }
}

fn platform_profile(w: &mut W, hw: &Hardware, pv: &PV) {
    let ch = &hw.cpu.platform_profiles;
    if ch.is_empty() || !hw.tlp.at_least(1, 5) {
        return;
    }
    w.section("Platform profile (firmware performance/thermal mode)");
    w.info(&format!("Firmware offers: {}", ch.join(" ")));
    if let (Some(a), Some(b)) = (pick(ch, pv.pp_ac), pick(ch, pv.pp_bat)) {
        let sav = pick(ch, w.sv.pp_bat).unwrap_or_else(|| b.clone());
        w.dual("PLATFORM_PROFILE", &a, &b, &sav);
    }
}

fn graphics(w: &mut W, hw: &Hardware, profile: Profile, warn: &mut Vec<String>) {
    let mut started = false;
    let mut begin = |w: &mut W| {
        if !started {
            w.section("Graphics");
            started = true;
        }
    };
    for g in &hw.gpus {
        match (g.vendor, g.driver.as_deref()) {
            (GpuVendor::Intel, Some("i915")) => {
                if let Some((rpn, rp1, rp0)) = g.intel_freqs {
                    begin(w);
                    w.info(&format!("Intel iGPU (i915): {}-{} MHz range.", rpn, rp0));
                    let bat_max = if profile == Profile::Battery {
                        rp1.unwrap_or(rp0)
                    } else {
                        rp0
                    };
                    let sav_max = rp1.unwrap_or(rp0);
                    let (rpn, rp0, bat_max, sav_max) = (
                        rpn.to_string(),
                        rp0.to_string(),
                        bat_max.to_string(),
                        sav_max.to_string(),
                    );
                    w.dual("INTEL_GPU_MIN_FREQ", &rpn, &rpn, &rpn);
                    w.dual("INTEL_GPU_MAX_FREQ", &rp0, &bat_max, &sav_max);
                    w.dual("INTEL_GPU_BOOST_FREQ", &rp0, &bat_max, &sav_max);
                }
            }
            (GpuVendor::Intel, Some("xe")) => {
                warn.push("Intel GPU uses the new 'xe' driver; TLP's INTEL_GPU_* frequency keys only support i915, so none were written.".into());
            }
            (GpuVendor::Amd, Some(d)) | (GpuVendor::Other, Some(d))
                if (d == "amdgpu" || d == "radeon") && g.dpm_level.is_some() =>
            {
                begin(w);
                w.info(&format!(
                    "{} ({}): DPM performance level is controllable.",
                    g.name, d
                ));
                // (RADEON_DPM_STATE_* was removed from TLP; the perf level works for radeon and amdgpu.)
                w.dual("RADEON_DPM_PERF_LEVEL", "auto", "auto", "auto");
            }
            (GpuVendor::Nvidia, Some("nvidia")) => {
                begin(w);
                w.info(&format!("NVIDIA proprietary driver on {}.", g.name));
                match g.nvidia_dyn_pm {
                    Some(0) => w.info("NVreg_DynamicPowerManagement is 0 (off): the card cannot runtime-suspend; TLP will leave it alone (see denylist below)."),
                    Some(n) => w.info(&format!("NVreg_DynamicPowerManagement={} - fine-grained runtime PM is enabled.", n)),
                    None => w.info("Could not read NVIDIA dynamic power management state."),
                }
                if g.role == "discrete" && g.nvidia_dyn_pm.is_none() {
                    warn.push("NVIDIA dGPU runtime power management state is unknown. For hybrid laptops, set options nvidia NVreg_DynamicPowerManagement=0x02 in /etc/modprobe.d/ so the GPU can power off.".into());
                }
            }
            _ => {}
        }
    }
}

fn storage(w: &mut W, hw: &Hardware, pv: &PV, warn: &mut Vec<String>) {
    let internal: Vec<&Disk> = hw
        .disks
        .iter()
        .filter(|d| !d.removable && d.transport != Transport::Usb)
        .collect();
    if internal.is_empty() {
        return;
    }
    w.section("Storage");
    for d in &internal {
        w.info(&format!(
            "{}: {} {} [{}, driver {}, scheduler {}]",
            d.name,
            d.kind(),
            d.model,
            d.transport.label(),
            d.driver.as_deref().unwrap_or("?"),
            d.sched.as_deref().unwrap_or("?")
        ));
    }
    let names: Vec<&str> = internal.iter().map(|d| d.name.as_str()).collect();
    w.info("Order of the values below follows DISK_DEVICES.");
    w.kv("DISK_DEVICES", &names.join(" "));

    // I/O scheduler: only values the kernel actually offers for each disk.
    let scheds: Vec<String> = internal
        .iter()
        .map(|d| {
            let choice = if d.transport == Transport::Nvme {
                pick(&d.scheds, &["none", "mq-deadline"])
            } else if d.rotational {
                pick(&d.scheds, &["bfq", "mq-deadline", "none"])
            } else {
                pick(&d.scheds, &["mq-deadline", "none", "bfq"])
            };
            choice.unwrap_or_else(|| "keep".into())
        })
        .collect();
    if scheds.iter().any(|s| s != "keep") {
        w.kv("DISK_IOSCHED", &scheds.join(" "));
    }

    // Spinning disks: APM and spindown (meaningless for SSD/NVMe).
    if internal.iter().any(|d| d.rotational) {
        let join = |f: &dyn Fn(&Disk) -> String| {
            internal.iter().map(|d| f(d)).collect::<Vec<_>>().join(" ")
        };
        w.info("HDD detected: advanced power management + spindown (needs hdparm). 'keep' leaves non-HDDs untouched.");
        let per_disk = |hdd: String| {
            join(&|d| {
                if d.rotational {
                    hdd.clone()
                } else {
                    "keep".into()
                }
            })
        };
        let (apm_bat, apm_sav) = (pv.hdd_apm_bat.to_string(), w.sv.hdd_apm_bat.to_string());
        let (spin_bat, spin_sav) = (pv.hdd_spin_bat.to_string(), w.sv.hdd_spin_bat.to_string());
        w.dual(
            "DISK_APM_LEVEL",
            &per_disk("254".into()),
            &per_disk(apm_bat),
            &per_disk(apm_sav),
        );
        w.dual(
            "DISK_SPINDOWN_TIMEOUT",
            &per_disk("0".into()),
            &per_disk(spin_bat),
            &per_disk(spin_sav),
        );
        if !hw.tlp.has_hdparm {
            warn.push("A spinning disk is present but 'hdparm' is not installed; TLP cannot apply HDD power settings without it.".into());
        }
    }

    // SATA link power management / AHCI runtime PM
    let sata_disks = internal.iter().any(|d| d.transport == Transport::Sata);
    if hw.sata_link_pm.is_some()
        && (sata_disks
            || internal
                .iter()
                .any(|d| d.driver.as_deref().map_or(false, |x| x.contains("ahci"))))
    {
        w.info("SATA link power management policy is exposed by the kernel.");
        w.dual(
            "SATA_LINKPWR",
            "max_performance",
            pv.sata_bat,
            w.sv.sata_bat,
        );
    }
    if internal
        .iter()
        .any(|d| d.driver.as_deref().map_or(false, |x| x.contains("ahci")))
    {
        w.dual("AHCI_RUNTIME_PM", "on", "auto", "auto");
        w.kv("AHCI_RUNTIME_PM_TIMEOUT", "15");
    }

    if internal.iter().any(|d| d.transport == Transport::Nvme) {
        if let Some(0) = hw.platform.nvme_apst_latency_us {
            warn.push("NVMe APST is disabled (nvme_core.default_ps_max_latency_us=0). The SSD will never enter low-power states; remove that kernel parameter if it was added as a workaround you no longer need.".into());
        }
    }
}

fn pcie(w: &mut W, hw: &Hardware, pv: &PV, warn: &mut Vec<String>) {
    w.section("PCIe & runtime power management");
    match &hw.platform.pcie_aspm {
        Some((choices, cur)) => {
            let bat = if choices.iter().any(|c| c == pv.aspm_bat) { pv.aspm_bat } else { "default" };
            w.info(&format!("PCIe ASPM policies available: {} (current: {}).", choices.join(" "), cur.as_deref().unwrap_or("?")));
            let sav = if choices.iter().any(|c| c == w.sv.aspm_bat) { w.sv.aspm_bat } else { "default" };
            w.dual("PCIE_ASPM", "default", bat, sav);
        }
        None => warn.push("PCIe ASPM policy control is not available (disabled by firmware or kernel). PCIE_ASPM_* was left out.".into()),
    }
    if hw.platform.cmdline.contains("pcie_aspm=off") {
        warn.push(
            "Kernel cmdline contains pcie_aspm=off, which prevents TLP from using ASPM.".into(),
        );
    }
    w.dual("RUNTIME_PM", "on", "auto", "auto");

    let mut deny = vec!["mei_me", "nouveau", "radeon", "xhci_hcd"];
    let nvidia_off = hw
        .gpus
        .iter()
        .any(|g| g.driver.as_deref() == Some("nvidia") && g.nvidia_dyn_pm == Some(0));
    if nvidia_off {
        deny.push("nvidia");
    }
    let key = if hw.tlp.at_least(1, 5) {
        "RUNTIME_PM_DRIVER_DENYLIST"
    } else {
        "RUNTIME_PM_DRIVER_BLACKLIST"
    };
    w.info("Drivers excluded from runtime PM (TLP defaults, plus anything this machine needs).");
    w.kv(key, &deny.join(" "));
}

fn radios(w: &mut W, hw: &Hardware, pv: &PV, warn: &mut Vec<String>) {
    let wifi: Vec<&NetIf> = hw.net.iter().filter(|n| n.kind == NetKind::Wifi).collect();
    let eth = hw.net.iter().any(|n| n.kind == NetKind::Ethernet);
    let has_bt = !hw.bluetooth.is_empty() || hw.usb.iter().any(|u| u.bluetooth);
    let wwan = hw.net.iter().any(|n| n.kind == NetKind::Wwan);
    if wifi.is_empty() && !eth && !has_bt && !wwan {
        return;
    }
    w.section("Network & radios");
    if !wifi.is_empty() {
        let d: Vec<String> = wifi
            .iter()
            .map(|n| format!("{} ({})", n.name, n.driver.as_deref().unwrap_or("?")))
            .collect();
        w.info(&format!("Wi-Fi: {}", d.join(", ")));
        w.dual("WIFI_PWR", "off", pv.wifi_bat, w.sv.wifi_bat);
        if !hw.tlp.has_iw {
            warn.push("Wi-Fi present but 'iw' is not installed; TLP cannot set Wi-Fi power saving without it.".into());
        }
    }
    if eth {
        w.info("Ethernet present: disable Wake-on-LAN to save power (set N if you use it).");
        w.kv("WOL_DISABLE", "Y");
        if !hw.tlp.has_ethtool {
            warn.push(
                "Ethernet present but 'ethtool' is not installed; WOL_DISABLE needs it.".into(),
            );
        }
    }
    if has_bt || wwan {
        let mut dev = vec![];
        if has_bt {
            dev.push("bluetooth");
        }
        if wwan {
            dev.push("wwan");
        }
        w.info("Optional: switch radios off on battery while they are not connected/in use.");
        match w.scheme {
            Scheme::Legacy => w.ckv("DEVICES_TO_DISABLE_ON_BAT_NOT_IN_USE", &dev.join(" ")),
            Scheme::Profiles => {
                w.ckv("DEVICES_TO_DISABLE_ON_BAL_NOT_IN_USE", &dev.join(" "));
                w.ckv("DEVICES_TO_DISABLE_ON_SAV_NOT_IN_USE", &dev.join(" "));
            }
        }
    }
}

fn usb(w: &mut W, hw: &Hardware) {
    if hw.usb.is_empty() {
        return;
    }
    w.section("USB");
    w.kv("USB_AUTOSUSPEND", "1");
    w.kv("USB_EXCLUDE_AUDIO", "1");
    w.kv("USB_EXCLUDE_BTUSB", "0");
    w.kv("USB_EXCLUDE_PHONE", "0");
    w.kv("USB_EXCLUDE_PRINTER", "1");
    w.kv("USB_EXCLUDE_WWAN", "0");
    // Plugged-in mice/keyboards stutter when autosuspended: keep them awake.
    let hid: Vec<&UsbDev> = hw
        .usb
        .iter()
        .filter(|u| u.hid && u.removable != "fixed")
        .collect();
    if !hid.is_empty() {
        let names: Vec<String> = hid
            .iter()
            .map(|u| format!("{} ({})", u.name, u.id))
            .collect();
        w.info(&format!(
            "External input devices kept awake: {}",
            names.join(", ")
        ));
        let ids: Vec<&str> = hid.iter().map(|u| u.id.as_str()).collect();
        let key = if hw.tlp.at_least(1, 5) {
            "USB_DENYLIST"
        } else {
            "USB_BLACKLIST"
        };
        w.kv(key, &ids.join(" "));
    }
}

fn audio(w: &mut W, hw: &Hardware) {
    if !hw.audio.hda && !hw.audio.ac97 {
        return;
    }
    w.section("Audio");
    w.info(&format!("Sound cards: {}", hw.audio.cards.join(", ")));
    w.dual("SOUND_POWER_SAVE", "0", "1", "1");
    if hw.audio.hda {
        w.kv("SOUND_POWER_SAVE_CONTROLLER", "Y");
    }
}

fn misc(w: &mut W, hw: &Hardware) {
    let mut any = false;
    if hw.platform.nmi_watchdog {
        w.section("Kernel");
        any = true;
        w.info("Disable the NMI watchdog (saves wake-ups; irrelevant unless you debug lockups).");
        w.kv("NMI_WATCHDOG", "0");
    }
    if let Some((choices, cur)) = &hw.platform.mem_sleep {
        if choices.len() > 1 {
            if !any {
                w.section("Kernel");
            }
            w.info(&format!(
                "Suspend modes offered by firmware: {} (current: {}).\nTLP can switch per power source; left commented because the right choice is machine specific.",
                choices.join(" "), cur.as_deref().unwrap_or("?")
            ));
            if choices.iter().any(|c| c == "deep") {
                w.dual_c("MEM_SLEEP", "s2idle", "deep", "deep");
            }
        }
    }
}

fn battery(w: &mut W, hw: &Hardware, pv: &PV, o: &Options, warn: &mut Vec<String>) {
    if hw.batteries.is_empty() {
        return;
    }
    w.section("Battery care");
    let wanted = match (o.start, o.stop) {
        (None, None) => pv.thresh,
        (s, e) => {
            let d = pv.thresh.unwrap_or((75, 80));
            Some((s.unwrap_or(d.0), e.unwrap_or(d.1)))
        }
    };
    let wanted = wanted.map(|(mut s, mut e)| {
        e = e.min(100);
        if s >= e {
            s = e.saturating_sub(5);
            warn.push(format!(
                "Start threshold must be below stop threshold; start adjusted to {}.",
                s
            ));
        }
        (s, e)
    });

    for b in &hw.batteries {
        let id = b.name.to_uppercase();
        let standard = id == "BAT0" || id == "BAT1";
        let mut line = format!("{}: ", b.name);
        if let Some(m) = &b.model {
            line.push_str(&format!("{} ", m));
        }
        if let Some(t) = &b.technology {
            line.push_str(&format!("({}) ", t));
        }
        if let Some(h) = b.health {
            line.push_str(&format!("health {:.0}%", h));
        }
        if let Some(c) = b.cycles {
            line.push_str(&format!(", {} cycles", c));
        }
        w.info(line.trim_end());

        if !b.has_stop {
            w.info("The kernel exposes no charge threshold control for this battery (driver/firmware unsupported); no thresholds written.");
            warn.push(format!("{}: no charge_control_end_threshold in sysfs - charge thresholds are not supported by this laptop's driver.", b.name));
            continue;
        }
        let pfx_ok = standard && hw.tlp.at_least(1, 0);
        if !standard {
            w.info(&format!("'{}' is not a standard TLP battery name (BAT0/BAT1); the keys below are commented. Check `tlp-stat -b` on your TLP version.", b.name));
        }
        let emit = |w: &mut W, k: &str, v: &str| if pfx_ok { w.kv(k, v) } else { w.ckv(k, v) };
        match wanted {
            Some((s, e)) => {
                if b.has_start {
                    emit(w, &format!("START_CHARGE_THRESH_{}", id), &s.to_string());
                }
                emit(w, &format!("STOP_CHARGE_THRESH_{}", id), &e.to_string());
                if !b.has_start {
                    w.info(
                        "Only an end threshold is supported here, so no start threshold is set.",
                    );
                }
            }
            None => {
                w.info(
                    "Performance profile keeps the battery fully charged. Uncomment to limit wear:",
                );
                w.ckv(&format!("START_CHARGE_THRESH_{}", id), "75");
                w.ckv(&format!("STOP_CHARGE_THRESH_{}", id), "80");
            }
        }
    }
    if let (Some((s, e)), true) = (wanted, hw.batteries.iter().any(|b| b.has_stop)) {
        w.info(&format!(
            "{}-{}% is a common longevity range; use `sudo tlp fullcharge` for a one-off 100%.",
            s, e
        ));
    }
}

fn environment_warnings(hw: &Hardware, warn: &mut Vec<String>) {
    if !hw.tlp.installed {
        warn.push("TLP does not appear to be installed (no 'tlp' binary in PATH). Install it first, e.g. 'sudo pacman -S tlp' / 'sudo apt install tlp'.".into());
    } else if let Some((a, b)) = hw.tlp.version {
        if (a, b) < (1, 5) {
            warn.push(format!("TLP {}.{} is older than 1.5: some keys (platform profile, *_DENYLIST) were adapted or skipped; consider updating.", a, b));
        }
    }
    for s in hw.services.iter().filter(|s| s.conflicts_with_tlp) {
        warn.push(format!(
            "'{}' is {} and conflicts with TLP. Disable it: sudo systemctl disable --now {}",
            s.name,
            if s.active { "running" } else { "enabled" },
            s.name
        ));
    }
    if hw.tlp.installed && !hw.tlp.has_dropin_dir {
        warn.push("/etc/tlp.d does not exist (very old TLP?). Append the output to /etc/tlp.conf instead.".into());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    use crate::demo::{battery as bat, disk, intel_gpu, strs, thinkpad as laptop};

    fn opts(p: Profile) -> Options {
        Options {
            profile: p,
            start: None,
            stop: None,
            terse: false,
        }
    }

    fn gen(hw: &Hardware, p: Profile) -> Generated {
        generate(hw, &opts(p))
    }

    fn has(text: &str, line: &str) -> bool {
        text.lines().any(|l| l == line)
    }

    // ---------- tests ----------

    #[test]
    fn balanced_laptop_cpu() {
        let g = gen(&laptop(), Profile::Balanced);
        let t = &g.text;
        assert!(has(t, r#"CPU_SCALING_GOVERNOR_ON_AC="powersave""#));
        assert!(has(t, r#"CPU_SCALING_GOVERNOR_ON_BAT="powersave""#));
        assert!(has(
            t,
            r#"CPU_ENERGY_PERF_POLICY_ON_AC="balance_performance""#
        ));
        assert!(has(t, r#"CPU_ENERGY_PERF_POLICY_ON_BAT="balance_power""#));
        assert!(has(t, r#"CPU_BOOST_ON_AC="1""#));
        assert!(has(t, r#"CPU_BOOST_ON_BAT="0""#));
        assert!(has(t, r#"CPU_HWP_DYN_BOOST_ON_BAT="0""#));
        assert!(has(t, r#"CPU_MAX_PERF_ON_BAT="100""#));
        assert!(has(t, r#"PLATFORM_PROFILE_ON_AC="performance""#));
        assert!(has(t, r#"PLATFORM_PROFILE_ON_BAT="low-power""#));
    }

    #[test]
    fn balanced_laptop_storage() {
        let t = gen(&laptop(), Profile::Balanced).text;
        assert!(has(&t, r#"DISK_DEVICES="nvme0n1 sda sdb""#));
        assert!(has(&t, r#"DISK_IOSCHED="none mq-deadline bfq""#));
        assert!(has(&t, r#"DISK_APM_LEVEL_ON_AC="keep keep 254""#));
        assert!(has(&t, r#"DISK_APM_LEVEL_ON_BAT="keep keep 128""#));
        assert!(has(&t, r#"DISK_SPINDOWN_TIMEOUT_ON_BAT="keep keep 120""#));
        assert!(has(&t, r#"SATA_LINKPWR_ON_BAT="med_power_with_dipm""#));
        assert!(has(&t, r#"AHCI_RUNTIME_PM_ON_BAT="auto""#));
    }

    #[test]
    fn ssd_only_machine_has_no_hdd_keys() {
        let mut hw = laptop();
        hw.disks.truncate(2);
        let t = gen(&hw, Profile::Balanced).text;
        assert!(!t.contains("DISK_APM_LEVEL"));
        assert!(!t.contains("DISK_SPINDOWN"));
        assert!(has(&t, r#"DISK_DEVICES="nvme0n1 sda""#));
    }

    #[test]
    fn usb_and_removable_disks_are_not_managed() {
        let mut hw = laptop();
        let mut usbdisk = disk("sdc", Transport::Usb, true, &["bfq"]);
        usbdisk.removable = true;
        hw.disks.push(usbdisk);
        let t = gen(&hw, Profile::Balanced).text;
        assert!(has(&t, r#"DISK_DEVICES="nvme0n1 sda sdb""#));
    }

    #[test]
    fn devices_pcie_audio_usb_misc() {
        let t = gen(&laptop(), Profile::Balanced).text;
        assert!(has(&t, r#"PCIE_ASPM_ON_BAT="powersave""#));
        assert!(has(&t, r#"WIFI_PWR_ON_BAT="on""#));
        assert!(has(&t, r#"WOL_DISABLE="Y""#));
        assert!(has(&t, r#"SOUND_POWER_SAVE_ON_BAT="1""#));
        assert!(has(&t, r#"SOUND_POWER_SAVE_CONTROLLER="Y""#));
        assert!(has(&t, r#"USB_AUTOSUSPEND="1""#));
        assert!(
            has(&t, r#"USB_DENYLIST="046d:c52b""#),
            "only the external HID device is excluded"
        );
        assert!(has(&t, r#"NMI_WATCHDOG="0""#));
        assert!(has(
            &t,
            r#"#DEVICES_TO_DISABLE_ON_BAT_NOT_IN_USE="bluetooth""#
        ));
        assert!(has(&t, r#"#MEM_SLEEP_ON_BAT="deep""#));
    }

    #[test]
    fn nvidia_without_dynamic_pm_is_denylisted() {
        let t = gen(&laptop(), Profile::Balanced).text;
        assert!(has(
            &t,
            r#"RUNTIME_PM_DRIVER_DENYLIST="mei_me nouveau radeon xhci_hcd nvidia""#
        ));
        let mut hw = laptop();
        hw.gpus[1].nvidia_dyn_pm = Some(3);
        let t = gen(&hw, Profile::Balanced).text;
        assert!(has(
            &t,
            r#"RUNTIME_PM_DRIVER_DENYLIST="mei_me nouveau radeon xhci_hcd""#
        ));
    }

    #[test]
    fn intel_igpu_frequencies_follow_profile() {
        let bal = gen(&laptop(), Profile::Balanced).text;
        assert!(has(&bal, r#"INTEL_GPU_MIN_FREQ_ON_AC="100""#));
        assert!(has(&bal, r#"INTEL_GPU_MAX_FREQ_ON_AC="1150""#));
        assert!(has(&bal, r#"INTEL_GPU_MAX_FREQ_ON_BAT="1150""#));
        let bat = gen(&laptop(), Profile::Battery).text;
        assert!(has(&bat, r#"INTEL_GPU_MAX_FREQ_ON_BAT="700""#));
    }

    #[test]
    fn amdgpu_and_xe_handling() {
        let mut hw = laptop();
        hw.gpus = vec![Gpu {
            vendor: GpuVendor::Amd,
            driver: Some("amdgpu".into()),
            dpm_level: Some("auto".into()),
            intel_freqs: None,
            ..intel_gpu()
        }];
        let t = gen(&hw, Profile::Balanced).text;
        assert!(has(&t, r#"RADEON_DPM_PERF_LEVEL_ON_BAT="auto""#));
        assert!(!t.contains("INTEL_GPU"));

        let mut hw = laptop();
        hw.gpus = vec![Gpu {
            driver: Some("xe".into()),
            intel_freqs: None,
            ..intel_gpu()
        }];
        let g = gen(&hw, Profile::Balanced);
        assert!(!g.text.contains("INTEL_GPU"));
        assert!(g.warnings.iter().any(|w| w.contains("xe")));
    }

    #[test]
    fn battery_profile_is_more_aggressive() {
        let t = gen(&laptop(), Profile::Battery).text;
        assert!(has(&t, r#"CPU_ENERGY_PERF_POLICY_ON_BAT="power""#));
        assert!(has(&t, r#"CPU_MAX_PERF_ON_BAT="60""#));
        assert!(has(&t, r#"START_CHARGE_THRESH_BAT0="40""#));
        assert!(has(&t, r#"STOP_CHARGE_THRESH_BAT0="80""#));
        assert!(has(&t, r#"PCIE_ASPM_ON_BAT="powersupersave""#));
    }

    #[test]
    fn performance_profile_keeps_battery_full_and_boost_on() {
        let t = gen(&laptop(), Profile::Performance).text;
        assert!(has(&t, r#"CPU_BOOST_ON_BAT="1""#));
        assert!(has(&t, r#"CPU_ENERGY_PERF_POLICY_ON_AC="performance""#));
        assert!(has(&t, r#"WIFI_PWR_ON_BAT="off""#));
        assert!(has(&t, r#"PCIE_ASPM_ON_BAT="default""#));
        assert!(
            !has(&t, r#"STOP_CHARGE_THRESH_BAT0="80""#),
            "thresholds must not be active"
        );
        assert!(
            has(&t, r#"#STOP_CHARGE_THRESH_BAT0="80""#),
            "but offered commented out"
        );
    }

    #[test]
    fn threshold_overrides_and_validation() {
        let hw = laptop();
        let g = generate(
            &hw,
            &Options {
                profile: Profile::Balanced,
                start: Some(20),
                stop: Some(90),
                terse: false,
            },
        );
        assert!(has(&g.text, r#"START_CHARGE_THRESH_BAT0="20""#));
        assert!(has(&g.text, r#"STOP_CHARGE_THRESH_BAT0="90""#));

        let g = generate(
            &hw,
            &Options {
                profile: Profile::Balanced,
                start: Some(90),
                stop: Some(80),
                terse: false,
            },
        );
        assert!(has(&g.text, r#"START_CHARGE_THRESH_BAT0="75""#));
        assert!(g.warnings.iter().any(|w| w.contains("Start threshold")));
    }

    #[test]
    fn battery_capabilities_are_respected() {
        // end threshold only (e.g. ASUS): no START key at all
        let mut hw = laptop();
        hw.batteries = vec![bat("BAT0", false, true)];
        let t = gen(&hw, Profile::Balanced).text;
        assert!(!t.contains("START_CHARGE_THRESH"));
        assert!(has(&t, r#"STOP_CHARGE_THRESH_BAT0="80""#));

        // no threshold support at all
        hw.batteries = vec![bat("BAT0", false, false)];
        let g = gen(&hw, Profile::Balanced);
        assert!(!g.text.contains("CHARGE_THRESH"));
        assert!(g.warnings.iter().any(|w| w.contains("not supported")));

        // two batteries
        hw.batteries = vec![bat("BAT0", true, true), bat("BAT1", true, true)];
        let t = gen(&hw, Profile::Balanced).text;
        assert!(has(&t, r#"STOP_CHARGE_THRESH_BAT0="80""#));
        assert!(has(&t, r#"STOP_CHARGE_THRESH_BAT1="80""#));

        // non-standard battery name is only suggested, never active
        hw.batteries = vec![bat("BATC", true, true)];
        let t = gen(&hw, Profile::Balanced).text;
        assert!(has(&t, r##"#STOP_CHARGE_THRESH_BATC="80""##));
        assert!(!has(&t, r#"STOP_CHARGE_THRESH_BATC="80""#));
    }

    #[test]
    fn desktop_is_pinned_to_ac() {
        let mut hw = laptop();
        hw.batteries.clear();
        hw.platform.chassis_is_portable = false;
        let g = gen(&hw, Profile::Balanced);
        assert!(has(&g.text, r#"TLP_PERSISTENT_DEFAULT="1""#));
        assert!(!g.text.contains("Battery care"));
        assert!(g.warnings.iter().any(|w| w.contains("No battery")));
    }

    #[test]
    fn amd_pstate_passive_uses_dynamic_governors() {
        let mut hw = laptop();
        hw.cpu = Cpu {
            model: "AMD Ryzen 7 5800U".into(),
            vendor: CpuVendor::Amd,
            driver: Some("amd-pstate".into()),
            governors: strs(&[
                "conservative",
                "ondemand",
                "userspace",
                "powersave",
                "performance",
                "schedutil",
            ]),
            amd_pstate: Some("passive".into()),
            boost_knob: Some("cpufreq/boost".into()),
            ..Default::default()
        };
        let t = gen(&hw, Profile::Balanced).text;
        assert!(has(&t, r#"CPU_SCALING_GOVERNOR_ON_AC="schedutil""#));
        assert!(has(&t, r#"CPU_SCALING_GOVERNOR_ON_BAT="schedutil""#));
        assert!(has(&t, r#"CPU_DRIVER_OPMODE_ON_AC="passive""#));
        assert!(
            !t.contains("CPU_ENERGY_PERF_POLICY"),
            "no EPP in passive mode"
        );
        assert!(
            !t.contains("CPU_MAX_PERF_ON"),
            "intel_pstate-only keys must not leak"
        );
    }

    #[test]
    fn amd_pstate_epp_uses_powersave() {
        let mut hw = laptop();
        hw.cpu = Cpu {
            vendor: CpuVendor::Amd,
            driver: Some("amd-pstate-epp".into()),
            governors: strs(&["performance", "powersave"]),
            epp_choices: strs(&[
                "default",
                "performance",
                "balance_performance",
                "balance_power",
                "power",
            ]),
            amd_pstate: Some("active".into()),
            ..Default::default()
        };
        let t = gen(&hw, Profile::Balanced).text;
        assert!(has(&t, r#"CPU_SCALING_GOVERNOR_ON_BAT="powersave""#));
        assert!(has(&t, r#"CPU_ENERGY_PERF_POLICY_ON_BAT="balance_power""#));
        assert!(has(&t, r#"CPU_DRIVER_OPMODE_ON_BAT="active""#));
    }

    #[test]
    fn legacy_acpi_cpufreq_warns_and_caps_frequency() {
        let mut hw = laptop();
        hw.cpu = Cpu {
            vendor: CpuVendor::Intel,
            driver: Some("acpi-cpufreq".into()),
            governors: strs(&[
                "conservative",
                "ondemand",
                "powersave",
                "performance",
                "schedutil",
            ]),
            max_khz: Some(3_000_000),
            ..Default::default()
        };
        let g = gen(&hw, Profile::Battery);
        assert!(g.warnings.iter().any(|w| w.contains("intel_pstate")));
        assert!(has(&g.text, r#"CPU_SCALING_MAX_FREQ_ON_BAT="1800000""#));
    }

    #[test]
    fn epp_value_falls_back_when_unsupported() {
        let mut hw = laptop();
        hw.cpu.epp_choices = strs(&["default", "performance", "power"]);
        let t = gen(&hw, Profile::Balanced).text;
        assert!(has(&t, r#"CPU_ENERGY_PERF_POLICY_ON_AC="default""#));
        assert!(has(&t, r#"CPU_ENERGY_PERF_POLICY_ON_BAT="default""#));
    }

    #[test]
    fn old_tlp_uses_blacklist_names_and_skips_platform_profile() {
        let mut hw = laptop();
        hw.tlp.version = Some((1, 4));
        let g = gen(&hw, Profile::Balanced);
        assert!(g.text.contains("RUNTIME_PM_DRIVER_BLACKLIST"));
        assert!(g.text.contains("USB_BLACKLIST"));
        assert!(!g.text.contains("PLATFORM_PROFILE"));
        assert!(g.warnings.iter().any(|w| w.contains("older than 1.5")));
    }

    #[test]
    fn conflicting_services_and_missing_tools_are_reported() {
        let mut hw = laptop();
        hw.services = vec![Service {
            name: "power-profiles-daemon".into(),
            active: true,
            enabled: true,
            conflicts_with_tlp: true,
        }];
        hw.tlp.installed = false;
        hw.tlp.has_hdparm = false;
        hw.tlp.has_iw = false;
        let w = gen(&hw, Profile::Balanced).warnings.join("\n");
        assert!(w.contains("power-profiles-daemon"));
        assert!(w.contains("hdparm"));
        assert!(w.contains("'iw'"));
        assert!(w.contains("not appear to be installed"));
    }

    #[test]
    fn disabled_nvme_apst_is_flagged() {
        let mut hw = laptop();
        hw.platform.nvme_apst_latency_us = Some(0);
        assert!(gen(&hw, Profile::Balanced)
            .warnings
            .iter()
            .any(|w| w.contains("APST")));
    }

    #[test]
    fn missing_hardware_emits_no_keys() {
        // A bare machine: nothing detectable except the bare minimum.
        let hw = Hardware {
            tlp: TlpInfo {
                installed: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let t = gen(&hw, Profile::Balanced).text;
        for absent in [
            "CPU_",
            "DISK_",
            "INTEL_GPU",
            "WIFI_",
            "USB_",
            "SOUND_",
            "CHARGE_THRESH",
            "PLATFORM_PROFILE",
        ] {
            assert!(!t.contains(absent), "{} should not be present", absent);
        }
    }

    #[test]
    fn output_is_well_formed_for_every_profile() {
        for (p, ver) in [Profile::Balanced, Profile::Performance, Profile::Battery]
            .into_iter()
            .flat_map(|p| [None, Some((1, 7)), Some((1, 11))].map(|v| (p, v)))
        {
            for terse in [false, true] {
                let mut hw = laptop();
                hw.tlp.version = ver;
                let t = generate(
                    &hw,
                    &Options {
                        profile: p,
                        start: None,
                        stop: None,
                        terse,
                    },
                )
                .text;
                let mut seen = HashSet::new();
                for line in t.lines() {
                    if line.is_empty() || line.starts_with('#') {
                        continue;
                    }
                    let (k, v) = line
                        .split_once('=')
                        .unwrap_or_else(|| panic!("not KEY=VALUE: {line}"));
                    assert!(
                        !k.is_empty()
                            && k.chars()
                                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_'),
                        "bad key: {line}"
                    );
                    assert!(
                        v.len() >= 2 && v.starts_with('"') && v.ends_with('"'),
                        "unquoted value: {line}"
                    );
                    assert!(seen.insert(k.to_string()), "duplicate key {k}");
                }
            }
        }
    }

    #[test]
    fn terse_mode_drops_explanations_only() {
        let verbose = gen(&laptop(), Profile::Balanced).text;
        let terse = generate(
            &laptop(),
            &Options {
                profile: Profile::Balanced,
                start: None,
                stop: None,
                terse: true,
            },
        )
        .text;
        assert!(terse.len() < verbose.len());
        let active = |t: &str| {
            t.lines()
                .filter(|l| !l.starts_with('#') && !l.is_empty())
                .count()
        };
        assert_eq!(
            active(&terse),
            active(&verbose),
            "terse must not change the settings"
        );
    }

    #[test]
    fn profile_parsing() {
        assert_eq!(Profile::parse("Balanced"), Some(Profile::Balanced));
        assert_eq!(Profile::parse("perf"), Some(Profile::Performance));
        assert_eq!(Profile::parse("saver"), Some(Profile::Battery));
        assert_eq!(Profile::parse("turbo"), None);
    }

    fn modern() -> Hardware {
        let mut hw = laptop();
        hw.tlp.version = Some((1, 11));
        hw.tlp.version_str = Some("1.11.0".into());
        hw
    }

    fn active_keys(text: &str) -> Vec<String> {
        text.lines()
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .filter_map(|l| l.split_once('=').map(|(k, _)| k.to_string()))
            .collect()
    }

    #[test]
    fn tlp_1_11_uses_profile_suffixes() {
        let t = gen(&modern(), Profile::Balanced).text;
        for base in [
            "CPU_SCALING_GOVERNOR",
            "CPU_ENERGY_PERF_POLICY",
            "CPU_BOOST",
            "CPU_HWP_DYN_BOOST",
            "CPU_MIN_PERF",
            "CPU_MAX_PERF",
            "PLATFORM_PROFILE",
            "INTEL_GPU_MIN_FREQ",
            "INTEL_GPU_MAX_FREQ",
            "INTEL_GPU_BOOST_FREQ",
            "DISK_APM_LEVEL",
            "DISK_SPINDOWN_TIMEOUT",
            "SATA_LINKPWR",
            "AHCI_RUNTIME_PM",
            "PCIE_ASPM",
            "RUNTIME_PM",
            "WIFI_PWR",
            "SOUND_POWER_SAVE",
        ] {
            for suffix in ["PRF", "BAL", "SAV"] {
                assert!(
                    t.contains(&format!("\n{base}_ON_{suffix}=")),
                    "missing {base}_ON_{suffix}"
                );
            }
        }
        let keys = active_keys(&t);
        assert!(
            !keys
                .iter()
                .any(|k| k.ends_with("_ON_AC") || k.ends_with("_ON_BAT")),
            "legacy suffix leaked: {keys:?}"
        );
    }

    #[test]
    fn classic_names_for_old_or_unknown_tlp() {
        for ver in [None, Some((1, 6)), Some((1, 10))] {
            let mut hw = laptop();
            hw.tlp.version = ver;
            let keys = active_keys(&gen(&hw, Profile::Balanced).text);
            assert!(
                keys.iter().any(|k| k == "CPU_BOOST_ON_AC"),
                "version {ver:?}"
            );
            assert!(
                keys.iter().any(|k| k == "CPU_BOOST_ON_BAT"),
                "version {ver:?}"
            );
            assert!(
                !keys.iter().any(|k| k.ends_with("_ON_PRF")
                    || k.ends_with("_ON_BAL")
                    || k.ends_with("_ON_SAV")),
                "version {ver:?}"
            );
        }
    }

    #[test]
    fn power_saver_profile_gets_the_aggressive_values() {
        let t = gen(&modern(), Profile::Balanced).text;
        assert!(has(&t, r#"CPU_ENERGY_PERF_POLICY_ON_BAL="balance_power""#));
        assert!(has(&t, r#"CPU_ENERGY_PERF_POLICY_ON_SAV="power""#));
        assert!(has(&t, r#"CPU_MAX_PERF_ON_BAL="100""#));
        assert!(has(&t, r#"CPU_MAX_PERF_ON_SAV="60""#));
        assert!(has(&t, r#"PCIE_ASPM_ON_BAL="powersave""#));
        assert!(has(&t, r#"PCIE_ASPM_ON_SAV="powersupersave""#));
        assert!(has(&t, r#"INTEL_GPU_MAX_FREQ_ON_BAL="1150""#));
        assert!(has(&t, r#"INTEL_GPU_MAX_FREQ_ON_SAV="700""#));
        assert!(has(&t, r#"DISK_APM_LEVEL_ON_PRF="keep keep 254""#));
        assert!(has(&t, r#"DISK_SPINDOWN_TIMEOUT_ON_SAV="keep keep 60""#));
        // The performance profile only changes plugged-in/balanced behaviour; saver stays a saver.
        let p = gen(&modern(), Profile::Performance).text;
        assert!(has(&p, r#"CPU_BOOST_ON_BAL="1""#));
        assert!(has(&p, r#"CPU_BOOST_ON_SAV="0""#));
    }

    #[test]
    fn desktop_pinning_depends_on_scheme() {
        let strip = |mut hw: Hardware| {
            hw.batteries.clear();
            hw.platform.chassis_is_portable = false;
            hw
        };
        let new = gen(&strip(modern()), Profile::Balanced).text;
        assert!(has(&new, r#"TLP_AUTO_SWITCH="0""#));
        assert!(has(&new, r#"TLP_PROFILE_DEFAULT="PRF""#));
        assert!(!new.contains("TLP_PERSISTENT_DEFAULT") && !new.contains("TLP_DEFAULT_MODE"));
        let old = gen(&strip(laptop()), Profile::Balanced).text;
        assert!(has(&old, r#"TLP_PERSISTENT_DEFAULT="1""#));
        assert!(has(&old, r#"TLP_DEFAULT_MODE="AC""#));
        assert!(!old.contains("TLP_AUTO_SWITCH"));
    }

    #[test]
    fn laptops_are_not_pinned() {
        for hw in [laptop(), modern()] {
            let keys = active_keys(&gen(&hw, Profile::Balanced).text);
            for k in [
                "TLP_DEFAULT_MODE",
                "TLP_PERSISTENT_DEFAULT",
                "TLP_AUTO_SWITCH",
                "TLP_PROFILE_DEFAULT",
            ] {
                assert!(
                    !keys.iter().any(|x| x == k),
                    "{k} must not be set on a laptop"
                );
            }
        }
    }

    #[test]
    fn removed_radeon_state_keys_are_never_emitted() {
        for hw in [laptop(), modern()] {
            let mut hw = hw;
            hw.gpus = vec![Gpu {
                vendor: GpuVendor::Amd,
                driver: Some("radeon".into()),
                dpm_level: Some("auto".into()),
                has_dpm_state: true,
                intel_freqs: None,
                ..intel_gpu()
            }];
            let t = gen(&hw, Profile::Balanced).text;
            assert!(t.contains("RADEON_DPM_PERF_LEVEL_ON_"));
            assert!(!t.contains("RADEON_DPM_STATE"));
        }
    }

    #[test]
    fn radio_hint_uses_the_right_name() {
        let t = gen(&modern(), Profile::Balanced).text;
        assert!(has(
            &t,
            r##"#DEVICES_TO_DISABLE_ON_BAL_NOT_IN_USE="bluetooth""##
        ));
        assert!(has(
            &t,
            r##"#DEVICES_TO_DISABLE_ON_SAV_NOT_IN_USE="bluetooth""##
        ));
        assert!(!t.contains("ON_BAT_NOT_IN_USE"));
    }

    #[test]
    fn frequency_cap_without_percentage_knob() {
        let mut hw = modern();
        hw.cpu = Cpu {
            vendor: CpuVendor::Intel,
            driver: Some("acpi-cpufreq".into()),
            governors: strs(&[
                "conservative",
                "ondemand",
                "powersave",
                "performance",
                "schedutil",
            ]),
            max_khz: Some(3_000_000),
            ..Default::default()
        };
        let bal = gen(&hw, Profile::Balanced).text;
        assert!(
            !bal.contains("CPU_SCALING_MAX_FREQ_ON_BAL"),
            "balanced does not cap"
        );
        assert!(has(&bal, r#"CPU_SCALING_MAX_FREQ_ON_SAV="1800000""#));
        let bat = gen(&hw, Profile::Battery).text;
        assert!(has(&bat, r#"CPU_SCALING_MAX_FREQ_ON_BAL="1800000""#));
    }

    #[test]
    fn header_states_which_key_style_was_used() {
        assert!(gen(&modern(), Profile::Balanced)
            .text
            .contains("profile-style keys"));
        assert!(gen(&laptop(), Profile::Balanced)
            .text
            .contains("classic keys"));
        let mut unknown = laptop();
        unknown.tlp.version = None;
        unknown.tlp.version_str = None;
        assert!(gen(&unknown, Profile::Balanced)
            .text
            .contains("version unknown"));
    }
}
