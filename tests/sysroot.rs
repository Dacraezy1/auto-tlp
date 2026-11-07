// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dacraezy1
//! Runs the real binary against fake machines (`--sysroot`). These are the tests of the *detection*
//! code: they prove that what the kernel exposes in sysfs/procfs turns into the right TLP settings.
mod common;
use common::*;

#[test]
fn thinkpad_cpu_and_platform() {
    let f = Fake::thinkpad();
    let o = f.run(&["-p", "balanced"]);
    assert!(o.status.success(), "{}", stderr(&o));
    let t = stdout(&o);
    assert!(
        t.contains("# Machine   : LENOVO ThinkPad X1 Fixture (Notebook)"),
        "{t}"
    );
    assert!(t.contains("i7-1185G7"));
    assert!(t.contains("# TLP       : 1.11.0 (profile-style keys"));
    assert!(
        t.contains("4 cores / 8 threads"),
        "SMT pairs must be counted as 4 cores"
    );
    for line in [
        r#"CPU_SCALING_GOVERNOR_ON_PRF="powersave""#,
        r#"CPU_ENERGY_PERF_POLICY_ON_PRF="balance_performance""#,
        r#"CPU_ENERGY_PERF_POLICY_ON_BAL="balance_power""#,
        r#"CPU_ENERGY_PERF_POLICY_ON_SAV="power""#,
        r#"CPU_BOOST_ON_PRF="1""#,
        r#"CPU_BOOST_ON_BAL="0""#,
        r#"CPU_HWP_DYN_BOOST_ON_BAL="0""#,
        r#"CPU_MAX_PERF_ON_SAV="60""#,
        r#"PLATFORM_PROFILE_ON_PRF="performance""#,
        r#"PLATFORM_PROFILE_ON_BAL="low-power""#,
    ] {
        assert!(has(&t, line), "missing {line}\n{t}");
    }
}

#[test]
fn thinkpad_battery_ignores_peripheral_batteries() {
    let f = Fake::thinkpad();
    let t = stdout(&f.run(&[]));
    assert!(has(&t, r#"START_CHARGE_THRESH_BAT0="75""#));
    assert!(has(&t, r#"STOP_CHARGE_THRESH_BAT0="80""#));
    assert!(
        !t.contains("hidpp"),
        "wireless mouse battery must be ignored"
    );
    assert!(t.contains("health 91%"), "health = full/design = 41/45");
    assert!(t.contains("212 cycles"));
}

#[test]
fn thinkpad_storage_is_classified_by_real_sysfs_layout() {
    let f = Fake::thinkpad();
    let t = stdout(&f.run(&[]));
    assert!(has(&t, r#"DISK_DEVICES="nvme0n1 sda sdb""#), "{t}");
    assert!(has(&t, r#"DISK_IOSCHED="none mq-deadline bfq""#));
    assert!(
        has(&t, r#"DISK_APM_LEVEL_ON_PRF="keep keep 254""#),
        "only the HDD gets APM"
    );
    assert!(has(&t, r#"DISK_APM_LEVEL_ON_BAL="keep keep 128""#));
    assert!(has(&t, r#"SATA_LINKPWR_ON_BAL="med_power_with_dipm""#));
    assert!(has(&t, r#"AHCI_RUNTIME_PM_ON_BAL="auto""#));
}

#[test]
fn thinkpad_graphics() {
    let f = Fake::thinkpad();
    let t = stdout(&f.run(&["-p", "balanced"]));
    assert!(has(&t, r#"INTEL_GPU_MIN_FREQ_ON_PRF="100""#));
    assert!(has(&t, r#"INTEL_GPU_MAX_FREQ_ON_PRF="1150""#));
    assert!(has(&t, r#"INTEL_GPU_MAX_FREQ_ON_SAV="700""#));
    assert!(
        has(
            &t,
            r#"RUNTIME_PM_DRIVER_DENYLIST="mei_me nouveau radeon xhci_hcd nvidia""#
        ),
        "NVIDIA with dynamic PM off must be denylisted\n{t}"
    );
    assert!(t.contains("# GPU       : Intel Corporation TigerLake-LP GT2 [Iris Xe Graphics] [integrated, driver: i915]"));
}

#[test]
fn thinkpad_network_usb_audio_kernel() {
    let f = Fake::thinkpad();
    let t = stdout(&f.run(&[]));
    assert!(has(&t, r#"WIFI_PWR_ON_BAL="on""#));
    assert!(has(&t, r#"WOL_DISABLE="Y""#));
    assert!(has(&t, r#"SOUND_POWER_SAVE_ON_BAL="1""#));
    assert!(has(&t, r#"SOUND_POWER_SAVE_CONTROLLER="Y""#));
    assert!(has(&t, r#"PCIE_ASPM_ON_BAL="powersave""#));
    assert!(has(&t, r#"NMI_WATCHDOG="0""#));
    assert!(has(&t, r#"USB_AUTOSUSPEND="1""#));
    // two identical external mice -> listed once; camera and internal Bluetooth are not touched
    assert!(has(&t, r#"USB_DENYLIST="046d:c52b""#), "{t}");
    assert!(t.contains("#DEVICES_TO_DISABLE_ON_BAL_NOT_IN_USE=\"bluetooth\""));
}

#[test]
fn report_describes_the_scanned_machine() {
    let f = Fake::thinkpad();
    let o = f.run(&["--report", "--verbose"]);
    assert!(o.status.success());
    let t = stdout(&o);
    for needle in [
        "LENOVO ThinkPad X1 Fixture",
        "NVMe SSD",
        "HDD",
        "driver=nvme",
        "driver=ahci",
        "Samsung SSD 980 PRO 1TB",
        "WDC WD10SPZX-24Z10",
        "TigerLake-LP GT2 [Iris Xe Graphics]",
        "NVIDIA Corporation GA106M",
        "TLP: 1.11.0",
        "L17M3P55",
        "iwlwifi",
        "e1000e",
        "046d:c52b",
        "Intel Bluetooth",
    ] {
        assert!(t.contains(needle), "report is missing {needle:?}\n{t}");
    }
}

#[test]
fn amd_desktop() {
    let f = Fake::new();
    f.base()
        .amd_cpu(16)
        .remove("/sys/class/dmi/id/chassis_type")
        .file("/sys/class/dmi/id/chassis_type", "3")
        .file("/sys/class/dmi/id/product_name", "Desktop Fixture")
        .nvme("[none] mq-deadline")
        .amd_gpu();
    let o = f.run(&[]);
    assert!(o.status.success());
    let t = stdout(&o);
    assert!(has(&t, r#"TLP_AUTO_SWITCH="0""#), "desktop is pinned\n{t}");
    assert!(has(&t, r#"TLP_PROFILE_DEFAULT="PRF""#));
    assert!(has(&t, r#"CPU_DRIVER_OPMODE_ON_PRF="active""#));
    assert!(has(&t, r#"CPU_SCALING_GOVERNOR_ON_BAL="powersave""#));
    assert!(has(&t, r#"CPU_BOOST_ON_PRF="1""#));
    assert!(has(&t, r#"RADEON_DPM_PERF_LEVEL_ON_PRF="auto""#));
    assert!(!t.contains("CHARGE_THRESH"), "no battery, no thresholds");
    assert!(
        !t.contains("CPU_MAX_PERF"),
        "intel_pstate-only keys must not appear for AMD"
    );
    assert!(stderr(&o).contains("No battery"));
}

#[test]
fn classic_keys_for_old_tlp() {
    let f = Fake::thinkpad();
    f.tlp_version("1.6.1");
    let t = stdout(&f.run(&[]));
    assert!(has(&t, r#"CPU_BOOST_ON_AC="1""#));
    assert!(has(&t, r#"CPU_BOOST_ON_BAT="0""#));
    assert!(!t.contains("_ON_PRF") && !t.contains("_ON_SAV"));
}

#[test]
fn missing_tlp_is_reported_and_uses_classic_keys() {
    let f = Fake::thinkpad();
    f.remove("/usr/bin/tlp").remove("/usr/share/tlp");
    let o = f.run(&[]);
    assert!(o.status.success());
    assert!(has(&stdout(&o), r#"CPU_BOOST_ON_AC="1""#));
    assert!(stderr(&o).contains("does not appear to be installed"));
}

#[test]
fn missing_helper_tools_are_reported() {
    let f = Fake::thinkpad();
    f.remove("/usr/bin/hdparm").remove("/usr/bin/iw");
    let e = stderr(&f.run(&[]));
    assert!(e.contains("hdparm"), "{e}");
    assert!(e.contains("'iw'"), "{e}");
}

#[test]
fn battery_with_only_an_end_threshold() {
    let f = Fake::new();
    f.base().intel_cpu(4).battery("BAT0", false, true);
    let t = stdout(&f.run(&[]));
    assert!(!t.contains("START_CHARGE_THRESH"));
    assert!(has(&t, r#"STOP_CHARGE_THRESH_BAT0="80""#));
}

#[test]
fn battery_without_threshold_support() {
    let f = Fake::new();
    f.base().intel_cpu(4).battery("BAT0", false, false);
    let o = f.run(&[]);
    assert!(!stdout(&o).contains("CHARGE_THRESH"));
    assert!(stderr(&o).contains("not supported"));
}

#[test]
fn two_batteries() {
    let f = Fake::new();
    f.base()
        .intel_cpu(4)
        .battery("BAT0", true, true)
        .battery("BAT1", true, true);
    let t = stdout(&f.run(&[]));
    assert!(has(&t, r#"STOP_CHARGE_THRESH_BAT0="80""#));
    assert!(has(&t, r#"STOP_CHARGE_THRESH_BAT1="80""#));
}

#[test]
fn i915_newer_sysfs_layout() {
    // Newer kernels moved the frequency files from the card dir to gt/gt0/rps_*.
    let f = Fake::new();
    f.base().intel_cpu(4).intel_gpu();
    let card = "/sys/devices/pci0000:00/0000:00:02.0/drm/card0";
    f.remove(&format!("{card}/gt_RPn_freq_mhz"))
        .remove(&format!("{card}/gt_RP1_freq_mhz"))
        .remove(&format!("{card}/gt_RP0_freq_mhz"))
        .file(&format!("{card}/gt/gt0/rps_RPn_freq_mhz"), "200")
        .file(&format!("{card}/gt/gt0/rps_RP1_freq_mhz"), "650")
        .file(&format!("{card}/gt/gt0/rps_RP0_freq_mhz"), "1300");
    let t = stdout(&f.run(&["-p", "balanced"]));
    assert!(has(&t, r#"INTEL_GPU_MIN_FREQ_ON_PRF="200""#), "{t}");
    assert!(has(&t, r#"INTEL_GPU_MAX_FREQ_ON_PRF="1300""#));
    assert!(has(&t, r#"INTEL_GPU_MAX_FREQ_ON_SAV="650""#));
}

#[test]
fn empty_machine_does_not_crash() {
    let f = Fake::new();
    let o = f.run(&[]);
    assert!(o.status.success(), "{}", stderr(&o));
    assert!(has(&stdout(&o), r#"TLP_ENABLE="1""#));
    let r = f.run(&["--report", "--verbose"]);
    assert!(r.status.success());
    assert!(stdout(&r).contains("== Platform =="));
}

#[test]
fn garbage_values_do_not_crash() {
    let f = Fake::thinkpad();
    f.file("/sys/class/power_supply/BAT0/capacity", "not-a-number")
        .file("/sys/class/power_supply/BAT0/energy_full", "")
        .file("/sys/class/power_supply/BAT0/energy_full_design", "0")
        .file("/sys/block/sda/size", "xyz")
        .file(
            "/sys/devices/system/cpu/cpu0/cpufreq/cpuinfo_max_freq",
            "fast",
        )
        .file("/proc/meminfo", "MemTotal: lots kB")
        .file("/sys/class/dmi/id/chassis_type", "banana");
    let o = f.run(&[]);
    assert!(o.status.success(), "{}", stderr(&o));
    let r = f.run(&["--report", "--verbose"]);
    assert!(r.status.success(), "{}", stderr(&r));
}

#[test]
fn tlp_conf_overrides_are_flagged() {
    let f = Fake::thinkpad();
    f.file(
        "/etc/tlp.conf",
        "# comment\nCPU_BOOST_ON_BAL=0\nUNRELATED=1\n",
    );
    let e = stderr(&f.run(&[]));
    assert!(
        e.contains("/etc/tlp.conf already sets CPU_BOOST_ON_BAL"),
        "{e}"
    );
}

#[test]
fn only_and_skip_sections() {
    let f = Fake::thinkpad();
    let t = stdout(&f.run(&["--only", "cpu,battery"]));
    assert!(t.contains("CPU_BOOST_ON_PRF") && t.contains("CHARGE_THRESH"));
    for absent in [
        "DISK_",
        "USB_",
        "SOUND_",
        "WIFI_",
        "PCIE_",
        "INTEL_GPU",
        "NMI_",
    ] {
        assert!(!t.contains(absent), "--only cpu,battery leaked {absent}");
    }
    let t = stdout(&f.run(&["--skip", "usb, audio"]));
    assert!(!t.contains("USB_") && !t.contains("SOUND_"));
    assert!(t.contains("DISK_DEVICES"));
}

#[test]
fn diff_detects_stale_and_matching_configs() {
    let f = Fake::thinkpad();
    f.file(
        "/etc/tlp.d/99-auto-tlp.conf",
        "TLP_ENABLE=\"1\"\nCPU_BOOST_ON_BAL=\"1\"\nOLD_KEY=\"x\"\n",
    );
    let o = f.run(&["--diff"]);
    assert_eq!(o.status.code(), Some(1), "differences => exit 1");
    let t = stdout(&o);
    assert!(t.contains("~ CPU_BOOST_ON_BAL: \"1\" -> \"0\""), "{t}");
    assert!(t.contains("- OLD_KEY=\"x\""));
    assert!(t.contains("+ DISK_DEVICES="));

    // install the generated config into the fixture -> no differences, exit 0
    let generated = stdout(&f.run(&[]));
    f.file("/etc/tlp.d/99-auto-tlp.conf", &generated);
    let o = f.run(&["--diff"]);
    assert_eq!(o.status.code(), Some(0), "{}", stdout(&o));
    assert!(stdout(&o).contains("No differences"));
}

#[test]
fn diff_without_installed_config_and_against_file() {
    let f = Fake::thinkpad();
    let o = f.run(&["--diff"]);
    assert_eq!(o.status.code(), Some(1));
    assert!(stderr(&o).contains("does not exist"));
    assert!(stdout(&o).contains("+ TLP_ENABLE=\"1\""));

    f.file("/tmp/other.conf", "TLP_ENABLE=\"1\"\n");
    let o = f.run(&["--diff", "--against", "/tmp/other.conf"]);
    assert_eq!(o.status.code(), Some(1));
    assert!(stdout(&o).contains("--- /tmp/other.conf"));
    let missing = f.run(&["--diff", "--against", "/tmp/nope.conf"]);
    assert_eq!(missing.status.code(), Some(2));
}

#[test]
fn sysroot_refuses_to_write_to_the_real_system() {
    let f = Fake::thinkpad();
    for flag in ["--install", "--uninstall"] {
        let o = f.run(&[flag]);
        assert_eq!(o.status.code(), Some(2), "{flag}");
        assert!(stderr(&o).contains("--sysroot"));
    }
    assert_eq!(f.run(&["--install"]).status.code(), Some(2));
    assert!(!f.read("/etc/tlp.d/99-auto-tlp.conf").contains("TLP_ENABLE"));
}

#[test]
fn sysroot_must_be_a_directory() {
    let o = std::process::Command::new(env!("CARGO_BIN_EXE_auto-tlp"))
        .args(["--sysroot", "/definitely/not/a/dir"])
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(2));
}

#[test]
fn environment_variable_sets_the_sysroot() {
    let f = Fake::thinkpad();
    let o = std::process::Command::new(env!("CARGO_BIN_EXE_auto-tlp"))
        .env("AUTO_TLP_SYSROOT", &f.root)
        .arg("--report")
        .output()
        .unwrap();
    assert!(stdout(&o).contains("ThinkPad X1 Fixture"));
}
