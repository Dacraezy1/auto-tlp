// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dacraezy1
mod demo;
mod gen;
mod hw;
mod report;
mod util;

use gen::{Options, Profile};
use std::collections::BTreeSet;
use std::process::exit;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const DROPIN: &str = "/etc/tlp.d/99-auto-tlp.conf";

fn usage() {
    println!(
        "auto-tlp {VERSION} - generate a TLP config tailored to this machine's hardware

USAGE:
    auto-tlp [OPTIONS]

OPTIONS:
    -p, --profile <NAME>     balanced (default) | performance | battery
        --start-thresh <N>   battery start-charge threshold (0-99)
        --stop-thresh <N>    battery stop-charge threshold (1-100)
    -o, --output <FILE>      write the config to FILE instead of stdout
        --install            write {DROPIN} (needs root)
    -r, --report             only print the detected hardware report
    -v, --verbose            with --report: also list all PCI/USB devices and sensors
        --terse              omit explanatory comments from the config
        --tlp-version <X.Y>  generate for this TLP version instead of the installed one
                             (>= 1.11 uses the new _ON_PRF/_ON_BAL/_ON_SAV key names)
        --demo               use a built-in sample laptop instead of scanning this machine
    -h, --help               show this help
    -V, --version            show version"
    );
}

fn die(msg: &str) -> ! {
    eprintln!("error: {}", msg);
    exit(2);
}

/// "1.11" or "1.11.0" -> (1, 11, "1.11.0")
fn parse_tlp_version(v: &str) -> (u32, u32, String) {
    let mut it = v.split('.');
    let num = |x: Option<&str>| x.and_then(|t| t.parse::<u32>().ok());
    match (num(it.next()), num(it.next())) {
        (Some(maj), Some(min)) => (maj, min, v.to_string()),
        _ => die("--tlp-version must look like 1.11 or 1.11.0"),
    }
}

fn main() {
    let mut profile = Profile::Balanced;
    let (mut start, mut stop) = (None, None);
    let (mut output, mut install, mut report, mut verbose, mut terse) =
        (None, false, false, false, false);
    let (mut demo, mut tlp_version) = (false, None);

    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let mut val = |name: &str| {
            args.next()
                .unwrap_or_else(|| die(&format!("{} needs a value", name)))
        };
        match a.as_str() {
            "-h" | "--help" => return usage(),
            "-V" | "--version" => return println!("auto-tlp {}", VERSION),
            "-p" | "--profile" => {
                let v = val("--profile");
                profile = Profile::parse(&v)
                    .unwrap_or_else(|| die("profile must be balanced, performance or battery"));
            }
            "--start-thresh" => {
                start = Some(
                    val(&a)
                        .parse::<u8>()
                        .ok()
                        .filter(|v| *v < 100)
                        .unwrap_or_else(|| die("--start-thresh must be 0-99")),
                )
            }
            "--stop-thresh" => {
                stop = Some(
                    val(&a)
                        .parse::<u8>()
                        .ok()
                        .filter(|v| (1..=100).contains(v))
                        .unwrap_or_else(|| die("--stop-thresh must be 1-100")),
                )
            }
            "-o" | "--output" => output = Some(val(&a)),
            "--install" => install = true,
            "-r" | "--report" => report = true,
            "-v" | "--verbose" => verbose = true,
            "--terse" => terse = true,
            "--demo" => demo = true,
            "--tlp-version" => tlp_version = Some(parse_tlp_version(&val(&a))),
            other => {
                usage();
                die(&format!("unknown option '{}'", other));
            }
        }
    }

    if !demo && !std::path::Path::new("/sys").exists() {
        die("this tool needs Linux (/sys not found)");
    }

    if demo && install {
        die("--demo uses a made-up machine and cannot be combined with --install");
    }
    let mut hw = hw::detect();
    if demo {
        // Keep the real TLP detection (so output matches the TLP you have), fake everything else.
        let tlp = std::mem::take(&mut hw.tlp);
        hw = demo::thinkpad();
        if tlp.installed {
            hw.tlp = tlp;
        }
    }
    if let Some((maj, min, text)) = tlp_version {
        hw.tlp.installed = true;
        hw.tlp.version = Some((maj, min));
        hw.tlp.version_str = Some(text);
    }
    if report {
        report::print(&hw, verbose);
        return;
    }

    let g = gen::generate(
        &hw,
        &Options {
            profile,
            start,
            stop,
            terse,
        },
    );

    // Keys we emit that are also active in /etc/tlp.conf (which wins over /etc/tlp.d).
    let mut warnings = g.warnings;
    if let Some(conf) = util::read("/etc/tlp.conf") {
        let active: BTreeSet<&str> = conf
            .lines()
            .filter(|l| !l.trim_start().starts_with('#'))
            .filter_map(|l| l.split_once('=').map(|(k, _)| k.trim()))
            .collect();
        let ours: BTreeSet<&str> = g
            .text
            .lines()
            .filter(|l| !l.starts_with('#'))
            .filter_map(|l| l.split_once('=').map(|(k, _)| k))
            .collect();
        let clash: Vec<&&str> = active.intersection(&ours).collect();
        if !clash.is_empty() {
            let names: Vec<&str> = clash.iter().map(|k| **k).collect();
            warnings.push(format!(
                "/etc/tlp.conf already sets {} and takes precedence over /etc/tlp.d drop-ins. Comment those lines out there.",
                names.join(", ")
            ));
        }
    }

    if install {
        if util::run("id", &["-u"]).as_deref() != Some("0") {
            die("--install needs root: run with sudo");
        }
        let _ = std::fs::create_dir_all("/etc/tlp.d");
        if std::path::Path::new(DROPIN).exists() {
            let _ = std::fs::copy(DROPIN, format!("{}.bak", DROPIN));
        }
        std::fs::write(DROPIN, &g.text)
            .unwrap_or_else(|e| die(&format!("cannot write {}: {}", DROPIN, e)));
        eprintln!("Wrote {}", DROPIN);
    } else if let Some(path) = output {
        std::fs::write(&path, &g.text)
            .unwrap_or_else(|e| die(&format!("cannot write {}: {}", path, e)));
        eprintln!("Wrote {}", path);
    } else {
        print!("{}", g.text);
    }

    if !warnings.is_empty() {
        eprintln!("\nNotes:");
        for w in &warnings {
            eprintln!("  * {}", w);
        }
    }
    if install {
        eprintln!("\nApply now with: sudo tlp start    Verify with: sudo tlp-stat -c  and  sudo tlp-stat -s");
    }
}
