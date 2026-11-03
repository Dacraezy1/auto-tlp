// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dacraezy1
//! Hardware detection. Everything here is read-only: sysfs, procfs, DMI and a
//! few harmless `systemctl`/`tlp --version` calls.
use crate::util::*;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

// ───────────────────────────── Platform ─────────────────────────────

const VENDOR_MODULES: &[&str] = &[
    "thinkpad_acpi",
    "ideapad_laptop",
    "asus_wmi",
    "asus_nb_wmi",
    "dell_laptop",
    "dell_smbios",
    "hp_wmi",
    "msi_ec",
    "msi_wmi",
    "system76_acpi",
    "samsung_laptop",
    "sony_laptop",
    "toshiba_acpi",
    "huawei_wmi",
    "lg_laptop",
    "acer_wmi",
    "tuxedo_keyboard",
    "tuxedo_io",
    "framework_laptop",
    "cros_ec_lpcs",
    "surface_aggregator",
    "fujitsu_laptop",
    "eeepc_laptop",
    "apple_gmux",
    "macsmc_power",
];

#[derive(Debug, Default)]
pub struct Platform {
    pub vendor: String,
    pub product: String,
    pub version: String,
    pub bios: String,
    pub chassis: String,
    pub chassis_is_portable: bool,
    pub vendor_modules: Vec<String>,
    pub mem_total_kb: Option<u64>,
    pub kernel: String,
    pub distro: String,
    pub cmdline: String,
    pub pcie_aspm: Option<(Vec<String>, Option<String>)>,
    pub mem_sleep: Option<(Vec<String>, Option<String>)>,
    pub nvme_apst_latency_us: Option<u64>,
    pub nmi_watchdog: bool,
    pub hwmon: Vec<String>,
    pub thermal_zones: Vec<String>,
}

fn chassis_info(t: u32) -> (&'static str, bool) {
    match t {
        3 => ("Desktop", false),
        4 => ("Low-profile desktop", false),
        5 => ("Pizza box", false),
        6 => ("Mini tower", false),
        7 => ("Tower", false),
        8 => ("Portable", true),
        9 => ("Laptop", true),
        10 => ("Notebook", true),
        11 => ("Handheld", true),
        13 => ("All-in-one", false),
        14 => ("Sub-notebook", true),
        15 => ("Space-saving", false),
        17 | 23 => ("Server", false),
        30 => ("Tablet", true),
        31 => ("Convertible", true),
        32 => ("Detachable", true),
        35 => ("Mini PC", false),
        36 => ("Stick PC", false),
        _ => ("Unknown", false),
    }
}

fn dmi(f: &str) -> String {
    read(format!("/sys/class/dmi/id/{}", f)).unwrap_or_default()
}

fn detect_platform() -> Platform {
    let chassis_t: Option<u32> =
        read("/sys/class/dmi/id/chassis_type").and_then(|s| s.parse().ok());
    let (cname, portable) = chassis_t.map(chassis_info).unwrap_or(("Unknown", false));
    let mem_total_kb = read("/proc/meminfo").and_then(|m| {
        m.lines()
            .find(|l| l.starts_with("MemTotal:"))
            .and_then(|l| l.split_whitespace().nth(1))
            .and_then(|v| v.parse().ok())
    });
    let distro = read("/etc/os-release")
        .and_then(|s| {
            s.lines().find_map(|l| {
                l.strip_prefix("PRETTY_NAME=")
                    .map(|v| v.trim_matches('"').to_string())
            })
        })
        .unwrap_or_default();
    let mut hwmon: BTreeSet<String> = BTreeSet::new();
    for h in list_dir("/sys/class/hwmon") {
        if let Some(n) = read(h.join("name")) {
            hwmon.insert(n);
        }
    }
    let mut zones: BTreeSet<String> = BTreeSet::new();
    for z in list_dir("/sys/class/thermal") {
        if base(&z).starts_with("thermal_zone") {
            if let Some(t) = read(z.join("type")) {
                zones.insert(t);
            }
        }
    }
    Platform {
        vendor: dmi("sys_vendor"),
        product: {
            let n = dmi("product_name");
            if n.is_empty() {
                dmi("board_name")
            } else {
                n
            }
        },
        version: dmi("product_version"),
        bios: format!("{} {}", dmi("bios_version"), dmi("bios_date"))
            .trim()
            .to_string(),
        chassis: cname.to_string(),
        chassis_is_portable: portable,
        vendor_modules: VENDOR_MODULES
            .iter()
            .filter(|m| module_loaded(m))
            .map(|m| m.to_string())
            .collect(),
        mem_total_kb,
        kernel: read("/proc/sys/kernel/osrelease").unwrap_or_default(),
        distro,
        cmdline: read("/proc/cmdline").unwrap_or_default(),
        pcie_aspm: read("/sys/module/pcie_aspm/parameters/policy").map(|s| parse_choices(&s)),
        mem_sleep: read("/sys/power/mem_sleep").map(|s| parse_choices(&s)),
        nvme_apst_latency_us: read_u64(
            "/sys/module/nvme_core/parameters/default_ps_max_latency_us",
        ),
        nmi_watchdog: exists("/proc/sys/kernel/nmi_watchdog"),
        hwmon: hwmon.into_iter().collect(),
        thermal_zones: zones.into_iter().collect(),
    }
}

// ───────────────────────────── CPU ─────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CpuVendor {
    Intel,
    Amd,
    #[default]
    Other,
}

#[derive(Debug, Default)]
pub struct Cpu {
    pub model: String,
    pub vendor: CpuVendor,
    pub logical: usize,
    pub physical: usize,
    pub driver: Option<String>,
    pub governors: Vec<String>,
    pub governor: Option<String>,
    pub epp_choices: Vec<String>,
    pub epp: Option<String>,
    pub epb: bool,
    pub min_khz: Option<u64>,
    pub max_khz: Option<u64>,
    pub boost_knob: Option<String>,
    pub intel_pstate: Option<String>,
    pub intel_perf_pct: bool,
    pub hwp: bool,
    pub hwp_dyn_boost: bool,
    pub amd_pstate: Option<String>,
    pub platform_profiles: Vec<String>,
    pub platform_profile: Option<String>,
}

fn detect_cpu() -> Cpu {
    let info = read("/proc/cpuinfo").unwrap_or_default();
    let field = |key: &str| -> Option<String> {
        info.lines()
            .find(|l| l.starts_with(key))
            .and_then(|l| l.split_once(':'))
            .map(|(_, v)| v.trim().to_string())
    };
    let model = field("model name")
        .or_else(|| field("Model"))
        .or_else(|| field("Hardware"))
        .unwrap_or_else(|| "Unknown CPU".into());
    let vendor = match field("vendor_id").unwrap_or_default().as_str() {
        "GenuineIntel" => CpuVendor::Intel,
        "AuthenticAMD" | "HygonGenuine" => CpuVendor::Amd,
        _ => CpuVendor::Other,
    };
    let hwp = field("flags")
        .map(|f| f.split_whitespace().any(|x| x == "hwp"))
        .unwrap_or(false);

    let mut logical = 0usize;
    let mut cores: BTreeSet<(String, String)> = BTreeSet::new();
    for d in list_dir("/sys/devices/system/cpu") {
        let n = base(&d);
        let is_cpu = n.strip_prefix("cpu").map_or(false, |r| {
            !r.is_empty() && r.chars().all(|c| c.is_ascii_digit())
        });
        if !is_cpu {
            continue;
        }
        logical += 1;
        let pkg = read(d.join("topology/physical_package_id")).unwrap_or_default();
        let core = read(d.join("topology/core_id")).unwrap_or_else(|| n.clone());
        cores.insert((pkg, core));
    }
    let physical = if cores.is_empty() {
        logical
    } else {
        cores.len()
    };

    let cf = "/sys/devices/system/cpu/cpu0/cpufreq";
    let (governors, _) = read(format!("{}/scaling_available_governors", cf))
        .map(|s| parse_choices(&s))
        .unwrap_or_default();
    let (epp_choices, _) = read(format!("{}/energy_performance_available_preferences", cf))
        .map(|s| parse_choices(&s))
        .unwrap_or_default();
    let boost_knob = if exists("/sys/devices/system/cpu/intel_pstate/no_turbo") {
        Some("intel_pstate/no_turbo".to_string())
    } else if exists("/sys/devices/system/cpu/cpufreq/boost") {
        Some("cpufreq/boost".to_string())
    } else if exists(format!("{}/boost", cf)) {
        Some("per-policy boost".to_string())
    } else {
        None
    };
    let (platform_profiles, _) = read("/sys/firmware/acpi/platform_profile_choices")
        .map(|s| parse_choices(&s))
        .unwrap_or_default();

    Cpu {
        model,
        vendor,
        logical,
        physical,
        driver: read(format!("{}/scaling_driver", cf)),
        governors,
        governor: read(format!("{}/scaling_governor", cf)),
        epp_choices,
        epp: read(format!("{}/energy_performance_preference", cf)),
        epb: exists("/sys/devices/system/cpu/cpu0/power/energy_perf_bias"),
        min_khz: read_u64(format!("{}/cpuinfo_min_freq", cf)),
        max_khz: read_u64(format!("{}/cpuinfo_max_freq", cf)),
        boost_knob,
        intel_pstate: read("/sys/devices/system/cpu/intel_pstate/status"),
        intel_perf_pct: exists("/sys/devices/system/cpu/intel_pstate/min_perf_pct"),
        hwp,
        hwp_dyn_boost: exists("/sys/devices/system/cpu/intel_pstate/hwp_dynamic_boost"),
        amd_pstate: read("/sys/devices/system/cpu/amd_pstate/status"),
        platform_profiles,
        platform_profile: read("/sys/firmware/acpi/platform_profile"),
    }
}

// ───────────────────────────── Power supplies ─────────────────────────────

#[derive(Debug)]
pub struct Battery {
    pub name: String,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub technology: Option<String>,
    pub status: Option<String>,
    pub capacity: Option<u64>,
    pub design: Option<u64>,
    pub full: Option<u64>,
    pub unit: &'static str,
    pub cycles: Option<u64>,
    pub health: Option<f64>,
    pub has_start: bool,
    pub has_stop: bool,
    pub cur_start: Option<u64>,
    pub cur_stop: Option<u64>,
    pub has_behaviour: bool,
}

#[derive(Debug)]
pub struct AcAdapter {
    pub name: String,
    pub kind: String,
    pub online: bool,
}

fn detect_power() -> (Vec<Battery>, Vec<AcAdapter>) {
    let mut bats = Vec::new();
    let mut acs = Vec::new();
    for d in list_dir("/sys/class/power_supply") {
        let name = base(&d);
        let ty = read(d.join("type")).unwrap_or_default();
        if read(d.join("scope")).as_deref() == Some("Device") {
            continue; // wireless mouse/keyboard/headset batteries
        }
        match ty.as_str() {
            "Battery" => {
                let (design, full, unit) = if let Some(dsg) = read_u64(d.join("energy_full_design"))
                {
                    (Some(dsg), read_u64(d.join("energy_full")), "Wh")
                } else {
                    (
                        read_u64(d.join("charge_full_design")),
                        read_u64(d.join("charge_full")),
                        "Ah",
                    )
                };
                let health = match (design, full) {
                    (Some(a), Some(b)) if a > 0 => Some(b as f64 * 100.0 / a as f64),
                    _ => None,
                };
                bats.push(Battery {
                    name,
                    manufacturer: read(d.join("manufacturer")),
                    model: read(d.join("model_name")),
                    technology: read(d.join("technology")),
                    status: read(d.join("status")),
                    capacity: read_u64(d.join("capacity")),
                    design,
                    full,
                    unit,
                    cycles: read_u64(d.join("cycle_count")).filter(|c| *c > 0),
                    health,
                    has_start: exists(d.join("charge_control_start_threshold")),
                    has_stop: exists(d.join("charge_control_end_threshold")),
                    cur_start: read_u64(d.join("charge_control_start_threshold")),
                    cur_stop: read_u64(d.join("charge_control_end_threshold")),
                    has_behaviour: exists(d.join("charge_behaviour")),
                });
            }
            "Mains" | "USB" => acs.push(AcAdapter {
                online: read(d.join("online")).as_deref() == Some("1"),
                name,
                kind: ty,
            }),
            _ => {}
        }
    }
    (bats, acs)
}

// ───────────────────────────── PCI helpers ─────────────────────────────

fn is_pci_addr(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 12 && b[4] == b':' && b[7] == b':' && b[10] == b'.'
}

/// Closest PCI function in a canonical sysfs path (the controller a device hangs off).
fn pci_ancestor(canon: &Path) -> Option<String> {
    canon
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .filter(|s| is_pci_addr(s))
        .last()
        .map(|s| s.to_string())
}

fn pci_driver(addr: &str) -> Option<String> {
    link_base(format!("/sys/bus/pci/devices/{}/driver", addr))
}

/// Optional human names from pci.ids (hwdata / pciutils).
struct PciDb(Option<String>);

impl PciDb {
    fn load() -> Self {
        for p in [
            "/usr/share/hwdata/pci.ids",
            "/usr/share/misc/pci.ids",
            "/usr/share/pci.ids",
            "/usr/share/hwdata/pci.ids.gz-unused",
        ] {
            if let Ok(s) = fs::read_to_string(p) {
                return PciDb(Some(s));
            }
        }
        PciDb(None)
    }

    fn lookup(&self, ven: &str, dev: &str) -> (String, String) {
        let ven = ven.trim_start_matches("0x").to_lowercase();
        let dev = dev.trim_start_matches("0x").to_lowercase();
        let fallback = (format!("[{}]", ven), format!("[{}]", dev));
        let Some(db) = &self.0 else { return fallback };
        let mut in_vendor = false;
        let mut vname = None;
        for line in db.lines() {
            if line.starts_with('#') || line.is_empty() {
                continue;
            }
            if !line.starts_with('\t') {
                if in_vendor {
                    break;
                }
                if line.get(..4) == Some(ven.as_str()) && line.get(4..6) == Some("  ") {
                    in_vendor = true;
                    vname = line.get(6..).map(|s| s.to_string());
                }
            } else if in_vendor && !line.starts_with("\t\t") && line.get(1..5) == Some(dev.as_str())
            {
                let dname = line.get(7..).unwrap_or("").to_string();
                return (vname.unwrap_or(fallback.0), dname);
            }
        }
        (vname.unwrap_or(fallback.0), fallback.1)
    }
}

fn pci_class_label(class: &str) -> &'static str {
    let c = class.trim_start_matches("0x");
    match c.get(..4).unwrap_or("") {
        "0108" => "NVMe controller",
        "0106" => "SATA controller",
        "0104" => "RAID controller",
        "0200" => "Ethernet",
        "0280" => "Network (Wi-Fi/other)",
        "0300" => "VGA/display",
        "0302" => "3D controller",
        "0380" => "Display (other)",
        "0401" | "0403" => "Audio",
        "0600" => "Host bridge",
        "0601" => "ISA bridge",
        "0604" => "PCI bridge",
        "0c03" => "USB controller",
        "0c05" => "SMBus",
        "0d11" => "Bluetooth",
        "1080" => "Encryption",
        "1100" => "Signal processing",
        "0805" => "SD host",
        "0c80" => "Serial bus",
        "0700" => "Serial controller",
        "0780" => "Communication",
        _ => "Other",
    }
}

#[derive(Debug)]
pub struct PciDev {
    pub addr: String,
    pub class: &'static str,
    pub vendor_name: String,
    pub device_name: String,
    pub ids: String,
    pub driver: Option<String>,
}

// ───────────────────────────── Storage ─────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transport {
    Nvme,
    Sata,
    Usb,
    Mmc,
    Virtio,
    Scsi,
    Other,
}

impl Transport {
    pub fn label(self) -> &'static str {
        match self {
            Transport::Nvme => "NVMe",
            Transport::Sata => "SATA",
            Transport::Usb => "USB",
            Transport::Mmc => "eMMC/SD",
            Transport::Virtio => "virtio",
            Transport::Scsi => "SCSI/SAS",
            Transport::Other => "other",
        }
    }
}

#[derive(Debug)]
pub struct Disk {
    pub name: String,
    pub model: String,
    pub firmware: Option<String>,
    pub size_bytes: u64,
    pub rotational: bool,
    pub removable: bool,
    pub transport: Transport,
    pub scheds: Vec<String>,
    pub sched: Option<String>,
    pub driver: Option<String>,
}

impl Disk {
    pub fn kind(&self) -> &'static str {
        match (self.transport, self.rotational) {
            (Transport::Nvme, _) => "NVMe SSD",
            (_, true) => "HDD",
            (Transport::Mmc, _) => "flash",
            _ => "SSD",
        }
    }
}

fn detect_disks() -> Vec<Disk> {
    const SKIP: &[&str] = &["loop", "ram", "zram", "dm-", "md", "sr", "fd", "nbd", "rbd"];
    let mut out = Vec::new();
    for d in list_dir("/sys/block") {
        let name = base(&d);
        if SKIP.iter().any(|p| name.starts_with(p)) {
            continue;
        }
        let sectors = read_u64(d.join("size")).unwrap_or(0);
        if sectors == 0 {
            continue;
        }
        let canon = fs::canonicalize(&d).unwrap_or_else(|_| d.clone());
        let cs = canon.to_string_lossy().to_string();
        let transport = if name.starts_with("nvme") {
            Transport::Nvme
        } else if cs.contains("/usb") {
            Transport::Usb
        } else if cs.contains("/ata") {
            Transport::Sata
        } else if name.starts_with("mmcblk") {
            Transport::Mmc
        } else if name.starts_with("vd") || cs.contains("/virtio") {
            Transport::Virtio
        } else if cs.contains("/host") {
            Transport::Scsi
        } else {
            Transport::Other
        };
        let dev = d.join("device");
        let vendor = read(dev.join("vendor")).unwrap_or_default();
        let model = read(dev.join("model")).unwrap_or_default();
        let model = match vendor.as_str() {
            "" | "ATA" => model,
            v if v.starts_with("0x") => model, // virtio exposes a raw PCI id here
            v => format!("{} {}", v, model),
        };
        let (scheds, sched) = read(d.join("queue/scheduler"))
            .map(|s| parse_choices(&s))
            .unwrap_or_default();
        out.push(Disk {
            driver: pci_ancestor(&canon).and_then(|a| pci_driver(&a)),
            model: if model.is_empty() {
                "unknown model".into()
            } else {
                model
            },
            firmware: read(dev.join("firmware_rev")).or_else(|| read(dev.join("rev"))),
            size_bytes: sectors * 512,
            // Virtual disks claim to be rotational regardless of the backing store.
            rotational: transport != Transport::Virtio
                && read(d.join("queue/rotational")).as_deref() == Some("1"),
            removable: read(d.join("removable")).as_deref() == Some("1"),
            transport,
            scheds,
            sched,
            name,
        });
    }
    out
}

// ───────────────────────────── GPU ─────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuVendor {
    Intel,
    Amd,
    Nvidia,
    Other,
}

#[derive(Debug)]
pub struct Gpu {
    pub addr: String,
    pub vendor: GpuVendor,
    pub name: String,
    pub driver: Option<String>,
    pub role: &'static str,
    pub card: Option<String>,
    pub dpm_level: Option<String>,
    pub has_dpm_state: bool,
    pub runtime_status: Option<String>,
    /// i915 only: (RPn, RP1, RP0) in MHz
    pub intel_freqs: Option<(u64, Option<u64>, u64)>,
    pub nvidia_dyn_pm: Option<u64>,
}

fn detect_gpus(db: &PciDb) -> Vec<Gpu> {
    let drm: Vec<_> = list_dir("/sys/class/drm")
        .into_iter()
        .filter(|p| {
            let n = base(p);
            n.starts_with("card") && !n.contains('-')
        })
        .collect();
    let mut gpus = Vec::new();
    for d in list_dir("/sys/bus/pci/devices") {
        let class = read(d.join("class")).unwrap_or_default();
        if !class.starts_with("0x03") {
            continue;
        }
        let addr = base(&d);
        let ven = read(d.join("vendor")).unwrap_or_default();
        let dev = read(d.join("device")).unwrap_or_default();
        let (vn, dn) = db.lookup(&ven, &dev);
        let vendor = match ven.as_str() {
            "0x8086" => GpuVendor::Intel,
            "0x1002" => GpuVendor::Amd,
            "0x10de" => GpuVendor::Nvidia,
            _ => GpuVendor::Other,
        };
        let driver = pci_driver(&addr);
        let card = drm
            .iter()
            .find(|c| link_base(c.join("device")).as_deref() == Some(addr.as_str()))
            .cloned();
        let cdev = card.as_ref().map(|c| c.join("device"));
        let dpm_level = cdev
            .as_ref()
            .and_then(|c| read(c.join("power_dpm_force_performance_level")));
        let has_dpm_state = cdev
            .as_ref()
            .map_or(false, |c| exists(c.join("power_dpm_state")));
        let intel_freqs = match (&card, driver.as_deref()) {
            (Some(c), Some("i915")) => {
                match (
                    read_u64(c.join("gt_RPn_freq_mhz")),
                    read_u64(c.join("gt_RP0_freq_mhz")),
                ) {
                    (Some(n), Some(p0)) => Some((n, read_u64(c.join("gt_RP1_freq_mhz")), p0)),
                    _ => None,
                }
            }
            _ => None,
        };
        let nvidia_dyn_pm = if driver.as_deref() == Some("nvidia") {
            read("/proc/driver/nvidia/params").and_then(|p| {
                p.lines()
                    .find(|l| l.contains("DynamicPowerManagement") && !l.contains("Video"))
                    .and_then(|l| l.rsplit(':').next())
                    .and_then(|v| v.trim().parse().ok())
            })
        } else {
            None
        };
        gpus.push((
            read(d.join("boot_vga")).as_deref() == Some("1"),
            Gpu {
                name: format!("{} {}", vn, dn),
                runtime_status: read(d.join("power/runtime_status")),
                card: card.as_ref().map(|c| base(c)),
                role: "unknown",
                addr,
                vendor,
                driver,
                dpm_level,
                has_dpm_state,
                intel_freqs,
                nvidia_dyn_pm,
            },
        ));
    }
    let n = gpus.len();
    gpus.into_iter()
        .map(|(boot, mut g)| {
            g.role = match g.vendor {
                GpuVendor::Intel => "integrated",
                GpuVendor::Nvidia => "discrete",
                GpuVendor::Amd if n > 1 => {
                    if boot {
                        "integrated"
                    } else {
                        "discrete"
                    }
                }
                _ => "unknown",
            };
            g
        })
        .collect()
}

// ───────────────────────────── Network, BT, audio, USB ─────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetKind {
    Wifi,
    Ethernet,
    Wwan,
    Other,
}

#[derive(Debug)]
pub struct NetIf {
    pub name: String,
    pub kind: NetKind,
    pub driver: Option<String>,
    pub bus: &'static str,
}

fn detect_net() -> Vec<NetIf> {
    let mut v = Vec::new();
    for d in list_dir("/sys/class/net") {
        let name = base(&d);
        if !exists(d.join("device")) {
            continue; // virtual interface
        }
        let driver = link_base(d.join("device/driver"));
        let canon = fs::canonicalize(d.join("device")).unwrap_or_default();
        let bus = if canon.to_string_lossy().contains("/usb") {
            "usb"
        } else {
            "pci/other"
        };
        let kind = if exists(d.join("wireless")) || exists(d.join("phy80211")) {
            NetKind::Wifi
        } else if name.starts_with("ww")
            || matches!(driver.as_deref(), Some("qmi_wwan") | Some("cdc_mbim"))
        {
            NetKind::Wwan
        } else if read(d.join("type")).as_deref() == Some("1") {
            NetKind::Ethernet
        } else {
            NetKind::Other
        };
        v.push(NetIf {
            name,
            kind,
            driver,
            bus,
        });
    }
    v
}

#[derive(Debug)]
pub struct Bluetooth {
    pub name: String,
    pub driver: Option<String>,
}

fn detect_bluetooth() -> Vec<Bluetooth> {
    list_dir("/sys/class/bluetooth")
        .into_iter()
        .filter(|p| base(p).starts_with("hci") && !base(p).contains(':'))
        .map(|p| Bluetooth {
            driver: link_base(p.join("device/driver")),
            name: base(&p),
        })
        .collect()
}

#[derive(Debug, Default)]
pub struct Audio {
    pub hda: bool,
    pub ac97: bool,
    pub cards: Vec<String>,
}

fn detect_audio() -> Audio {
    Audio {
        hda: exists("/sys/module/snd_hda_intel/parameters/power_save"),
        ac97: exists("/sys/module/snd_ac97_codec/parameters/power_save"),
        cards: list_dir("/sys/class/sound")
            .into_iter()
            .filter(|p| {
                let n = base(p);
                n.starts_with("card") && n[4..].chars().all(|c| c.is_ascii_digit())
            })
            .filter_map(|p| read(p.join("id")))
            .collect(),
    }
}

#[derive(Debug)]
pub struct UsbDev {
    pub id: String,
    pub name: String,
    pub hid: bool,
    pub bluetooth: bool,
    pub removable: String,
}

fn detect_usb() -> Vec<UsbDev> {
    let mut v = Vec::new();
    for d in list_dir("/sys/bus/usb/devices") {
        let name = base(&d);
        if name.contains(':') {
            continue; // interface node
        }
        let (Some(vid), Some(pid)) = (read(d.join("idVendor")), read(d.join("idProduct"))) else {
            continue;
        };
        if read(d.join("bDeviceClass")).as_deref() == Some("09") {
            continue; // hubs
        }
        let prefix = format!("{}:", name);
        let classes: Vec<String> = list_dir(&d)
            .into_iter()
            .filter(|p| base(p).starts_with(&prefix))
            .filter_map(|p| read(p.join("bInterfaceClass")))
            .collect();
        let product = read(d.join("product")).unwrap_or_else(|| "USB device".into());
        let maker = read(d.join("manufacturer")).unwrap_or_default();
        v.push(UsbDev {
            id: format!("{}:{}", vid, pid),
            name: format!("{} {}", maker, product).trim().to_string(),
            hid: classes.iter().any(|c| c == "03"),
            bluetooth: classes.iter().any(|c| c == "e0"),
            removable: read(d.join("removable")).unwrap_or_else(|| "unknown".into()),
        });
    }
    v
}

// ───────────────────────────── Services & TLP ─────────────────────────────

#[derive(Debug)]
pub struct Service {
    pub name: String,
    pub active: bool,
    pub enabled: bool,
    pub conflicts_with_tlp: bool,
}

fn detect_services() -> Vec<Service> {
    if !command_exists("systemctl") {
        return Vec::new();
    }
    const WATCH: &[(&str, bool)] = &[
        ("tlp", false),
        ("power-profiles-daemon", true),
        ("tuned", true),
        ("tuned-ppd", true),
        ("auto-cpufreq", true),
        ("laptop-mode", true),
        ("system76-power", true),
        ("thermald", false),
        ("upower", false),
        ("cpupower", false),
    ];
    WATCH
        .iter()
        .filter_map(|(n, conflict)| {
            let active = run("systemctl", &["is-active", n]).as_deref() == Some("active");
            let enabled = run("systemctl", &["is-enabled", n]).as_deref() == Some("enabled");
            if active || enabled {
                Some(Service {
                    name: n.to_string(),
                    active,
                    enabled,
                    conflicts_with_tlp: *conflict,
                })
            } else {
                None
            }
        })
        .collect()
}

#[derive(Debug, Default)]
pub struct TlpInfo {
    pub installed: bool,
    pub version: Option<(u32, u32)>,
    pub version_str: Option<String>,
    pub has_hdparm: bool,
    pub has_iw: bool,
    pub has_ethtool: bool,
    pub has_dropin_dir: bool,
}

impl TlpInfo {
    /// TLP 1.11 introduced profile-style keys (`*_ON_PRF/BAL/SAV`). Only true when the version is known:
    /// the classic `*_ON_AC/BAT` names are accepted by every TLP version, the new ones only by >= 1.11.
    pub fn uses_profiles(&self) -> bool {
        self.version.is_some_and(|v| v >= (1, 11))
    }

    /// True if the installed TLP is at least `maj.min` (or if unknown: assume modern).
    pub fn at_least(&self, maj: u32, min: u32) -> bool {
        self.version.map_or(true, |v| v >= (maj, min))
    }
}

/// `readonly TLPVER="1.6.1"` as found in TLP's `tlp-func-base`.
fn parse_tlpver(text: &str) -> Option<String> {
    text.lines().find_map(|l| {
        let l = l.trim().trim_start_matches("readonly ").trim();
        l.strip_prefix("TLPVER=")
            .map(|v| v.trim_matches(|c| c == '"' || c == '\'').to_string())
            .filter(|v| v.chars().next().is_some_and(|c| c.is_ascii_digit()))
    })
}

/// `--- TLP 1.6.1 ----` header printed by `tlp-stat`.
fn parse_tlp_stat_header(text: &str) -> Option<String> {
    text.lines().find_map(|l| {
        let rest = l.trim_start().strip_prefix("--- TLP ")?;
        let v = rest.split_whitespace().next()?;
        v.chars().next()?.is_ascii_digit().then(|| v.to_string())
    })
}

fn tlp_version_string() -> Option<String> {
    for p in [
        "/usr/share/tlp/tlp-func-base",
        "/usr/share/tlp-pm/tlp-func-base",
        "/usr/lib/tlp-pm/tlp-functions",
    ] {
        if let Some(v) = read(p).and_then(|c| parse_tlpver(&c)) {
            return Some(v);
        }
    }
    run("tlp-stat", &["-c"]).and_then(|o| parse_tlp_stat_header(&o))
}

fn detect_tlp() -> TlpInfo {
    let installed = command_exists("tlp");
    let version_str = if installed {
        tlp_version_string()
    } else {
        None
    };
    let version = version_str.as_ref().and_then(|v| {
        let mut it = v.split('.');
        let maj = it.next()?.parse().ok()?;
        let min = it
            .next()?
            .trim_end_matches(|c: char| !c.is_ascii_digit())
            .parse()
            .ok()?;
        Some((maj, min))
    });
    TlpInfo {
        installed,
        version,
        version_str,
        has_hdparm: command_exists("hdparm"),
        has_iw: command_exists("iw"),
        has_ethtool: command_exists("ethtool"),
        has_dropin_dir: exists("/etc/tlp.d"),
    }
}

// ───────────────────────────── Aggregate ─────────────────────────────

#[derive(Debug, Default)]
pub struct Hardware {
    pub platform: Platform,
    pub cpu: Cpu,
    pub batteries: Vec<Battery>,
    pub ac: Vec<AcAdapter>,
    pub disks: Vec<Disk>,
    pub sata_link_pm: Option<String>,
    pub gpus: Vec<Gpu>,
    pub net: Vec<NetIf>,
    pub bluetooth: Vec<Bluetooth>,
    pub audio: Audio,
    pub usb: Vec<UsbDev>,
    pub services: Vec<Service>,
    pub tlp: TlpInfo,
    pub pci: Vec<PciDev>,
}

impl Hardware {
    /// Portable form factor, or a battery is present.
    pub fn is_laptop(&self) -> bool {
        self.platform.chassis_is_portable || !self.batteries.is_empty()
    }
}

pub fn detect() -> Hardware {
    let db = PciDb::load();
    let (batteries, ac) = detect_power();
    let pci = list_dir("/sys/bus/pci/devices")
        .into_iter()
        .map(|d| {
            let ven = read(d.join("vendor")).unwrap_or_default();
            let dev = read(d.join("device")).unwrap_or_default();
            let (vendor_name, device_name) = db.lookup(&ven, &dev);
            PciDev {
                addr: base(&d),
                class: pci_class_label(&read(d.join("class")).unwrap_or_default()),
                ids: format!(
                    "{}:{}",
                    ven.trim_start_matches("0x"),
                    dev.trim_start_matches("0x")
                ),
                driver: pci_driver(&base(&d)),
                vendor_name,
                device_name,
            }
        })
        .collect();
    let sata_link_pm = list_dir("/sys/class/scsi_host")
        .into_iter()
        .find_map(|h| read(h.join("link_power_management_policy")));
    Hardware {
        platform: detect_platform(),
        cpu: detect_cpu(),
        batteries,
        ac,
        disks: detect_disks(),
        sata_link_pm,
        gpus: detect_gpus(&db),
        net: detect_net(),
        bluetooth: detect_bluetooth(),
        audio: detect_audio(),
        usb: detect_usb(),
        services: detect_services(),
        tlp: detect_tlp(),
        pci,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PCI_IDS: &str = "# comment\n\
8086  Intel Corporation\n\
\t0046  Core Processor Integrated Graphics Controller\n\
\t9a49  TigerLake-LP GT2 [Iris Xe Graphics]\n\
\t\t17aa 22c1  subsystem line that must be ignored\n\
10de  NVIDIA Corporation\n\
\t2520  GA106M [GeForce RTX 3060 Mobile / Max-Q]\n\
C 03  Display controller\n";

    #[test]
    fn pci_lookup_resolves_vendor_and_device() {
        let db = PciDb(Some(PCI_IDS.to_string()));
        let (v, d) = db.lookup("0x8086", "0x9a49");
        assert_eq!(v, "Intel Corporation");
        assert_eq!(d, "TigerLake-LP GT2 [Iris Xe Graphics]");
        let (v, d) = db.lookup("0x10DE", "0x2520");
        assert_eq!(v, "NVIDIA Corporation");
        assert!(d.contains("RTX 3060"));
    }

    #[test]
    fn pci_lookup_falls_back_gracefully() {
        let db = PciDb(Some(PCI_IDS.to_string()));
        let (v, d) = db.lookup("0x8086", "0xffff");
        assert_eq!(v, "Intel Corporation");
        assert_eq!(d, "[ffff]");
        let (v, d) = db.lookup("0x1234", "0x5678");
        assert_eq!((v.as_str(), d.as_str()), ("[1234]", "[5678]"));
        let none = PciDb(None);
        assert_eq!(none.lookup("0x8086", "0x1").0, "[8086]");
    }

    #[test]
    fn pci_address_detection() {
        assert!(is_pci_addr("0000:00:02.0"));
        assert!(is_pci_addr("0000:3d:00.0"));
        assert!(!is_pci_addr("nvme0n1"));
        assert!(!is_pci_addr("pci0000:00"));
        let p = Path::new("/sys/devices/pci0000:00/0000:00:1d.0/0000:3d:00.0/nvme/nvme0/nvme0n1");
        assert_eq!(pci_ancestor(p).as_deref(), Some("0000:3d:00.0"));
    }

    #[test]
    fn pci_classes_are_labelled() {
        assert_eq!(pci_class_label("0x010802"), "NVMe controller");
        assert_eq!(pci_class_label("0x030000"), "VGA/display");
        assert_eq!(pci_class_label("0x028000"), "Network (Wi-Fi/other)");
        assert_eq!(pci_class_label("0xffffff"), "Other");
    }

    #[test]
    fn chassis_types() {
        assert_eq!(chassis_info(10), ("Notebook", true));
        assert_eq!(chassis_info(3), ("Desktop", false));
        assert_eq!(chassis_info(999), ("Unknown", false));
    }

    #[test]
    fn tlp_version_parsing() {
        let base = "#!/bin/sh\nreadonly TLPVER=\"1.6.1\"\nreadonly TLPPATH=x\n";
        assert_eq!(parse_tlpver(base).as_deref(), Some("1.6.1"));
        assert_eq!(parse_tlpver("TLPVER='1.8.0'").as_deref(), Some("1.8.0"));
        assert_eq!(parse_tlpver("TLPVER=\"\""), None);
        assert_eq!(parse_tlpver("nothing here"), None);
        let stat = "Failed to connect\n--- TLP 1.7.0 -----------\n\n+++ System Info\n";
        assert_eq!(parse_tlp_stat_header(stat).as_deref(), Some("1.7.0"));
        assert_eq!(parse_tlp_stat_header("garbage"), None);
    }

    #[test]
    fn tlp_version_comparison() {
        let mut t = TlpInfo::default();
        assert!(t.at_least(1, 9), "unknown version is assumed modern");
        t.version = Some((1, 4));
        assert!(!t.at_least(1, 5));
        assert!(t.at_least(1, 4));
        t.version = Some((1, 7));
        assert!(t.at_least(1, 5));
    }

    #[test]
    fn disk_kind_labels() {
        let mk = |t, rot| Disk {
            name: "x".into(),
            model: "m".into(),
            firmware: None,
            size_bytes: 1,
            rotational: rot,
            removable: false,
            transport: t,
            scheds: vec![],
            sched: None,
            driver: None,
        };
        assert_eq!(mk(Transport::Nvme, false).kind(), "NVMe SSD");
        assert_eq!(mk(Transport::Sata, true).kind(), "HDD");
        assert_eq!(mk(Transport::Sata, false).kind(), "SSD");
    }
}
