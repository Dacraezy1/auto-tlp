// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dacraezy1
//! Builds fake `/sys` + `/proc` trees that mimic what real laptops expose (including the kernel's
//! relative-symlink layout), so the detection code can be tested end to end with `--sysroot`.
#![allow(dead_code)]

use std::fs;
use std::os::unix::fs::{symlink, PermissionsExt};
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU32, Ordering};

static COUNTER: AtomicU32 = AtomicU32::new(0);

pub struct Fake {
    pub root: PathBuf,
}

impl Drop for Fake {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

pub fn has(text: &str, line: &str) -> bool {
    text.lines().any(|l| l == line)
}

pub fn stdout(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

pub fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

impl Fake {
    pub fn new() -> Fake {
        let root = std::env::temp_dir().join(format!(
            "auto-tlp-fake-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        ));
        fs::create_dir_all(&root).unwrap();
        Fake { root }
    }

    fn path(&self, p: &str) -> PathBuf {
        self.root.join(p.trim_start_matches('/'))
    }

    pub fn file(&self, p: &str, content: &str) -> &Self {
        let path = self.path(p);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, format!("{content}\n")).unwrap();
        self
    }

    pub fn dir(&self, p: &str) -> &Self {
        fs::create_dir_all(self.path(p)).unwrap();
        self
    }

    /// Relative symlink, like sysfs uses everywhere. Replaces an existing link.
    pub fn link(&self, p: &str, target: &str) -> &Self {
        let path = self.path(p);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let _ = fs::remove_file(&path);
        symlink(target, path).unwrap();
        self
    }

    pub fn exe(&self, p: &str) -> &Self {
        let path = self.path(p);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "#!/bin/sh\n").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        self
    }

    pub fn remove(&self, p: &str) -> &Self {
        let path = self.path(p);
        if path.is_dir() {
            let _ = fs::remove_dir_all(path);
        } else {
            let _ = fs::remove_file(path);
        }
        self
    }

    pub fn read(&self, p: &str) -> String {
        fs::read_to_string(self.path(p)).unwrap_or_default()
    }

    pub fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_auto-tlp"))
            .arg("--sysroot")
            .arg(&self.root)
            .args(args)
            .output()
            .expect("failed to run auto-tlp")
    }

    // ------------------------------------------------------------------ building blocks

    /// DMI, kernel, TLP 1.11 + helper tools, a tiny pci.ids.
    pub fn base(&self) -> &Self {
        self.file("/sys/class/dmi/id/sys_vendor", "LENOVO")
            .file("/sys/class/dmi/id/product_name", "ThinkPad X1 Fixture")
            .file("/sys/class/dmi/id/product_version", "Gen 9")
            .file("/sys/class/dmi/id/chassis_type", "10")
            .file("/sys/class/dmi/id/bios_version", "N32ET80W")
            .file("/sys/class/dmi/id/bios_date", "01/01/2026")
            .file("/proc/sys/kernel/osrelease", "6.12.1-fixture")
            .file("/proc/sys/kernel/nmi_watchdog", "1")
            .file("/proc/meminfo", "MemTotal:       16253144 kB")
            .file("/proc/cmdline", "root=/dev/nvme0n1p2 quiet")
            .file("/etc/os-release", "PRETTY_NAME=\"Fixture Linux\"")
            .file(
                "/sys/module/pcie_aspm/parameters/policy",
                "[default] performance powersave powersupersave",
            )
            .file("/sys/power/mem_sleep", "s2idle [deep]")
            .exe("/usr/bin/tlp")
            .exe("/usr/bin/hdparm")
            .exe("/usr/bin/iw")
            .exe("/usr/bin/ethtool")
            .dir("/etc/tlp.d")
            .tlp_version("1.11.0")
            .file(
                "/usr/share/hwdata/pci.ids",
                "8086  Intel Corporation\n\
                 \t9a49  TigerLake-LP GT2 [Iris Xe Graphics]\n\
                 144d  Samsung Electronics Co Ltd\n\
                 \ta80a  NVMe SSD Controller PM9A1\n\
                 10de  NVIDIA Corporation\n\
                 \t2520  GA106M [GeForce RTX 3060 Mobile]\n\
                 1002  Advanced Micro Devices, Inc. [AMD/ATI]\n\
                 \t73bf  Navi 21 [Radeon RX 6800]",
            )
    }

    pub fn tlp_version(&self, v: &str) -> &Self {
        self.file(
            "/usr/share/tlp/tlp-func-base",
            &format!("#!/bin/sh\nreadonly TLPVER=\"{v}\"\n"),
        )
    }

    /// Intel CPU with intel_pstate (active, HWP) and `n` logical CPUs (SMT pairs).
    pub fn intel_cpu(&self, n: usize) -> &Self {
        let mut info = String::new();
        for i in 0..n {
            info.push_str(&format!(
                "processor\t: {i}\nvendor_id\t: GenuineIntel\nmodel name\t: 11th Gen Intel(R) Core(TM) i7-1185G7 @ 3.00GHz\nflags\t\t: fpu vme hwp hwp_epp\n\n"
            ));
            let cpu = format!("/sys/devices/system/cpu/cpu{i}");
            self.file(&format!("{cpu}/topology/physical_package_id"), "0")
                .file(&format!("{cpu}/topology/core_id"), &(i / 2).to_string());
        }
        self.file("/proc/cpuinfo", &info);
        let cf = "/sys/devices/system/cpu/cpu0/cpufreq";
        self.file(&format!("{cf}/scaling_driver"), "intel_pstate")
            .file(
                &format!("{cf}/scaling_available_governors"),
                "performance powersave",
            )
            .file(&format!("{cf}/scaling_governor"), "powersave")
            .file(
                &format!("{cf}/energy_performance_available_preferences"),
                "default performance balance_performance balance_power power",
            )
            .file(
                &format!("{cf}/energy_performance_preference"),
                "balance_performance",
            )
            .file(&format!("{cf}/cpuinfo_min_freq"), "400000")
            .file(&format!("{cf}/cpuinfo_max_freq"), "4800000")
            .file("/sys/devices/system/cpu/intel_pstate/status", "active")
            .file("/sys/devices/system/cpu/intel_pstate/no_turbo", "0")
            .file("/sys/devices/system/cpu/intel_pstate/min_perf_pct", "8")
            .file(
                "/sys/devices/system/cpu/intel_pstate/hwp_dynamic_boost",
                "0",
            )
            .file("/sys/firmware/acpi/platform_profile", "balanced")
            .file(
                "/sys/firmware/acpi/platform_profile_choices",
                "low-power balanced performance",
            )
    }

    /// AMD Zen CPU with amd-pstate in active (EPP) mode.
    pub fn amd_cpu(&self, n: usize) -> &Self {
        let mut info = String::new();
        for i in 0..n {
            info.push_str(&format!(
                "processor\t: {i}\nvendor_id\t: AuthenticAMD\nmodel name\t: AMD Ryzen 7 5800X 8-Core Processor\nflags\t\t: fpu vme\n\n"
            ));
            let cpu = format!("/sys/devices/system/cpu/cpu{i}");
            self.file(&format!("{cpu}/topology/physical_package_id"), "0")
                .file(&format!("{cpu}/topology/core_id"), &(i / 2).to_string());
        }
        self.file("/proc/cpuinfo", &info);
        let cf = "/sys/devices/system/cpu/cpu0/cpufreq";
        self.file(&format!("{cf}/scaling_driver"), "amd-pstate-epp")
            .file(
                &format!("{cf}/scaling_available_governors"),
                "performance powersave",
            )
            .file(&format!("{cf}/scaling_governor"), "powersave")
            .file(
                &format!("{cf}/energy_performance_available_preferences"),
                "default performance balance_performance balance_power power",
            )
            .file(
                &format!("{cf}/energy_performance_preference"),
                "balance_performance",
            )
            .file(&format!("{cf}/cpuinfo_min_freq"), "400000")
            .file(&format!("{cf}/cpuinfo_max_freq"), "4850000")
            .file("/sys/devices/system/cpu/amd_pstate/status", "active")
            .file("/sys/devices/system/cpu/cpufreq/boost", "1")
    }

    /// A laptop battery (+ AC adapter and a wireless-mouse battery that must be ignored).
    pub fn battery(&self, name: &str, start: bool, stop: bool) -> &Self {
        let b = format!("/sys/class/power_supply/{name}");
        self.file(&format!("{b}/type"), "Battery")
            .file(&format!("{b}/status"), "Discharging")
            .file(&format!("{b}/capacity"), "80")
            .file(&format!("{b}/energy_full_design"), "45000000")
            .file(&format!("{b}/energy_full"), "41000000")
            .file(&format!("{b}/cycle_count"), "212")
            .file(&format!("{b}/manufacturer"), "SMP")
            .file(&format!("{b}/model_name"), "L17M3P55")
            .file(&format!("{b}/technology"), "Li-poly");
        if start {
            self.file(&format!("{b}/charge_control_start_threshold"), "0");
        }
        if stop {
            self.file(&format!("{b}/charge_control_end_threshold"), "100");
        }
        self.file("/sys/class/power_supply/AC/type", "Mains")
            .file("/sys/class/power_supply/AC/online", "1")
            .file("/sys/class/power_supply/hidpp_battery_0/type", "Battery")
            .file("/sys/class/power_supply/hidpp_battery_0/scope", "Device")
            .file("/sys/class/power_supply/hidpp_battery_0/capacity", "55")
    }

    /// A PCI function with the symlink layout sysfs uses.
    pub fn pci(
        &self,
        parent: &str,
        addr: &str,
        class: &str,
        vendor: &str,
        device: &str,
        driver: &str,
    ) -> &Self {
        let dir = format!("/sys/devices/{parent}/{addr}");
        self.file(&format!("{dir}/class"), class)
            .file(&format!("{dir}/vendor"), vendor)
            .file(&format!("{dir}/device"), device)
            .link(
                &format!("{dir}/driver"),
                &format!("../../../../bus/pci/drivers/{driver}"),
            )
            .link(
                &format!("/sys/bus/pci/devices/{addr}"),
                &format!("../../../devices/{parent}/{addr}"),
            )
    }

    pub fn nvme(&self, scheds: &str) -> &Self {
        let pci = "pci0000:00/0000:00:1d.0/0000:3d:00.0";
        self.pci(
            "pci0000:00/0000:00:1d.0",
            "0000:3d:00.0",
            "0x010802",
            "0x144d",
            "0xa80a",
            "nvme",
        );
        let ctrl = format!("/sys/devices/{pci}/nvme/nvme0");
        self.file(&format!("{ctrl}/model"), "Samsung SSD 980 PRO 1TB")
            .file(&format!("{ctrl}/firmware_rev"), "5B2QGXA7")
            .file(&format!("{ctrl}/nvme0n1/size"), "1953525168")
            .file(&format!("{ctrl}/nvme0n1/removable"), "0")
            .file(&format!("{ctrl}/nvme0n1/queue/rotational"), "0")
            .file(&format!("{ctrl}/nvme0n1/queue/scheduler"), scheds)
            .link(&format!("{ctrl}/nvme0n1/device"), "../../nvme0")
            .link(
                "/sys/block/nvme0n1",
                &format!("../devices/{pci}/nvme/nvme0/nvme0n1"),
            )
    }

    /// A SATA disk behind the AHCI controller at 0000:00:17.0.
    pub fn sata(
        &self,
        name: &str,
        host: u32,
        rotational: bool,
        scheds: &str,
        model: &str,
    ) -> &Self {
        self.pci(
            "pci0000:00",
            "0000:00:17.0",
            "0x010601",
            "0x8086",
            "0xa0d3",
            "ahci",
        );
        let scsi = format!(
            "/sys/devices/pci0000:00/0000:00:17.0/ata{}/host{host}/target{host}:0:0/{host}:0:0:0",
            host + 1
        );
        self.file(&format!("{scsi}/vendor"), "ATA     ")
            .file(&format!("{scsi}/model"), model)
            .file(&format!("{scsi}/rev"), "1B6Q")
            .file(&format!("{scsi}/block/{name}/size"), "976773168")
            .file(&format!("{scsi}/block/{name}/removable"), "0")
            .file(
                &format!("{scsi}/block/{name}/queue/rotational"),
                if rotational { "1" } else { "0" },
            )
            .file(&format!("{scsi}/block/{name}/queue/scheduler"), scheds)
            .link(&format!("{scsi}/block/{name}/device"), "../..")
            .link(
                &format!("/sys/block/{name}"),
                &format!(
                    "../devices/pci0000:00/0000:00:17.0/ata{}/host{host}/target{host}:0:0/{host}:0:0:0/block/{name}",
                    host + 1
                ),
            )
            .file(
                &format!("/sys/class/scsi_host/host{host}/link_power_management_policy"),
                "max_performance",
            )
    }

    /// Intel iGPU (i915) with a DRM card and the legacy frequency files.
    pub fn intel_gpu(&self) -> &Self {
        self.pci(
            "pci0000:00",
            "0000:00:02.0",
            "0x030000",
            "0x8086",
            "0x9a49",
            "i915",
        );
        let pci = "/sys/devices/pci0000:00/0000:00:02.0";
        self.file(&format!("{pci}/boot_vga"), "1")
            .file(&format!("{pci}/power/runtime_status"), "active")
            .file(&format!("{pci}/drm/card0/gt_RPn_freq_mhz"), "100")
            .file(&format!("{pci}/drm/card0/gt_RP1_freq_mhz"), "700")
            .file(&format!("{pci}/drm/card0/gt_RP0_freq_mhz"), "1150")
            .link(&format!("{pci}/drm/card0/device"), "../../../0000:00:02.0")
            .link(
                "/sys/class/drm/card0",
                "../../devices/pci0000:00/0000:00:02.0/drm/card0",
            )
            .dir("/sys/class/drm/card0-eDP-1") // connector entries must be ignored
    }

    /// NVIDIA dGPU with the proprietary driver; `dyn_pm` is NVreg_DynamicPowerManagement.
    pub fn nvidia_gpu(&self, dyn_pm: u32) -> &Self {
        self.pci("pci0000:00/0000:00:01.0", "0000:01:00.0", "0x030200", "0x10de", "0x2520", "nvidia")
            .file(
                "/proc/driver/nvidia/params",
                &format!("EnableMSI: 1\nDynamicPowerManagement: {dyn_pm}\nDynamicPowerManagementVideoMemoryThreshold: 200"),
            )
    }

    /// AMD dGPU (amdgpu) with DPM control.
    pub fn amd_gpu(&self) -> &Self {
        self.pci(
            "pci0000:00",
            "0000:03:00.0",
            "0x030000",
            "0x1002",
            "0x73bf",
            "amdgpu",
        );
        let pci = "/sys/devices/pci0000:00/0000:03:00.0";
        self.file(&format!("{pci}/power_dpm_force_performance_level"), "auto")
            .link(&format!("{pci}/drm/card1/device"), "../../../0000:03:00.0")
            .link(
                "/sys/class/drm/card1",
                "../../devices/pci0000:00/0000:03:00.0/drm/card1",
            )
    }

    fn net(&self, name: &str, pci_addr: &str, driver: &str, wireless: bool) -> &Self {
        let pci_dir = format!("/sys/devices/pci0000:00/{pci_addr}");
        self.file(
            &format!("{pci_dir}/class"),
            if wireless { "0x028000" } else { "0x020000" },
        )
        .file(&format!("{pci_dir}/vendor"), "0x8086")
        .file(&format!("{pci_dir}/device"), "0x1234")
        .link(
            &format!("{pci_dir}/driver"),
            &format!("../../../bus/pci/drivers/{driver}"),
        )
        .file(&format!("{pci_dir}/net/{name}/type"), "1")
        .link(
            &format!("{pci_dir}/net/{name}/device"),
            &format!("../../../{pci_addr}"),
        )
        .link(
            &format!("/sys/class/net/{name}"),
            &format!("../../devices/pci0000:00/{pci_addr}/net/{name}"),
        );
        if wireless {
            self.dir(&format!("{pci_dir}/net/{name}/wireless"));
        }
        self
    }

    pub fn wifi(&self) -> &Self {
        self.net("wlp0s20f3", "0000:00:14.3", "iwlwifi", true)
    }

    pub fn ethernet(&self) -> &Self {
        self.net("enp0s31f6", "0000:00:1f.6", "e1000e", false)
    }

    pub fn audio(&self) -> &Self {
        self.file("/sys/module/snd_hda_intel/parameters/power_save", "1")
            .file("/sys/class/sound/card0/id", "PCH")
    }

    pub fn bluetooth(&self) -> &Self {
        self.link(
            "/sys/class/bluetooth/hci0/device/driver",
            "../../../../../bus/usb/drivers/btusb",
        )
    }

    /// A USB device; `iface_class` is the first interface's class (03 = HID, e0 = Bluetooth).
    pub fn usb(
        &self,
        node: &str,
        vid: &str,
        pid: &str,
        name: &str,
        iface_class: &str,
        removable: &str,
    ) -> &Self {
        let d = format!("/sys/bus/usb/devices/{node}");
        self.file(&format!("{d}/idVendor"), vid)
            .file(&format!("{d}/idProduct"), pid)
            .file(&format!("{d}/product"), name)
            .file(&format!("{d}/manufacturer"), "Fixture Inc")
            .file(&format!("{d}/bDeviceClass"), "00")
            .file(&format!("{d}/removable"), removable)
            // interface nodes are *children* of the device directory in real sysfs
            .file(&format!("{d}/{node}:1.0/bInterfaceClass"), iface_class)
    }

    /// The full ThinkPad-like machine used by most tests.
    pub fn thinkpad() -> Fake {
        let f = Fake::new();
        f.base()
            .intel_cpu(8)
            .battery("BAT0", true, true)
            .nvme("[none] mq-deadline")
            .sata(
                "sda",
                0,
                false,
                "[mq-deadline] none",
                "Samsung SSD 870 EVO 500GB",
            )
            .sata(
                "sdb",
                1,
                true,
                "mq-deadline [bfq] none",
                "WDC WD10SPZX-24Z10",
            )
            .intel_gpu()
            .nvidia_gpu(0)
            .wifi()
            .ethernet()
            .bluetooth()
            .audio()
            // a hub (ignored), an internal camera, an internal BT radio, two identical external mice
            .file("/sys/bus/usb/devices/usb1/idVendor", "1d6b")
            .file("/sys/bus/usb/devices/usb1/idProduct", "0002")
            .file("/sys/bus/usb/devices/usb1/bDeviceClass", "09")
            .usb("1-5", "04f2", "b6d9", "Integrated Camera", "0e", "fixed")
            .usb("1-7", "8087", "0033", "Intel Bluetooth", "e0", "fixed")
            .usb("1-2", "046d", "c52b", "USB Receiver", "03", "removable")
            .usb("1-3", "046d", "c52b", "USB Receiver", "03", "removable");
        f
    }
}
