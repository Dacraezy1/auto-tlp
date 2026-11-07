// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dacraezy1
mod demo;
mod diff;
mod format;
mod gen;
mod hw;
mod report;
mod util;

use format::Format;
use gen::{Options, Profile, SECTIONS};
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::exit;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const DROPIN: &str = "/etc/tlp.d/99-auto-tlp.conf";

fn usage() {
    println!(
        "auto-tlp {VERSION} - generate a TLP config tailored to this machine's hardware

USAGE:
    auto-tlp [OPTIONS]

GENERATE:
    -p, --profile <NAME>     balanced (default) | performance | battery
        --start-thresh <N>   battery start-charge threshold (0-99)
        --stop-thresh <N>    battery stop-charge threshold (1-100)
        --only <LIST>        only these sections (comma separated, see below)
        --skip <LIST>        leave these sections out
        --terse              omit explanatory comments from the config
        --format <FMT>       conf (default) | nix (NixOS services.tlp snippet) | json
        --tlp-version <X.Y>  generate for this TLP version instead of the installed one
                             (>= 1.11 uses the new _ON_PRF/_ON_BAL/_ON_SAV key names)

OUTPUT:
    -o, --output <FILE>      write the config to FILE instead of stdout ('-' = stdout)
        --install            write {DROPIN} (needs root; keeps a .bak)
        --uninstall          remove {DROPIN} (needs root)
        --apply              with --install/--uninstall: run `tlp start` afterwards
        --diff               show what would change compared to the installed config
                             (exit status 1 if there are differences)
        --against <FILE>     with --diff: compare with FILE instead of the installed config

INSPECT:
    -r, --report             only print the detected hardware report
    -v, --verbose            with --report: also list all PCI/USB devices and sensors
        --demo               use a built-in sample laptop instead of scanning this machine
        --sysroot <DIR>      scan a directory tree instead of the live system (testing, bug reports)

    -h, --help               show this help
    -V, --version            show version

SECTIONS (for --only / --skip):
    {}",
        SECTIONS.join(", ")
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

fn parse_sections(list: &str) -> Vec<String> {
    list.split(',')
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .map(|s| {
            if SECTIONS.contains(&s.as_str()) {
                s
            } else {
                die(&format!(
                    "unknown section '{}'. Valid sections: {}",
                    s,
                    SECTIONS.join(", ")
                ))
            }
        })
        .collect()
}

#[derive(Default)]
struct Cli {
    profile: Profile,
    start: Option<u8>,
    stop: Option<u8>,
    output: Option<String>,
    install: bool,
    uninstall: bool,
    apply: bool,
    diff: bool,
    against: Option<String>,
    report: bool,
    verbose: bool,
    terse: bool,
    demo: bool,
    tlp_version: Option<(u32, u32, String)>,
    only: Vec<String>,
    skip: Vec<String>,
    sysroot: Option<String>,
    format: Format,
}

fn parse_args() -> Cli {
    let mut cli = Cli::default();
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let mut val = || {
            args.next()
                .unwrap_or_else(|| die(&format!("{} needs a value", a)))
        };
        match a.as_str() {
            "-h" | "--help" => {
                usage();
                exit(0);
            }
            "-V" | "--version" => {
                println!("auto-tlp {}", VERSION);
                exit(0);
            }
            "-p" | "--profile" => {
                cli.profile = Profile::parse(&val())
                    .unwrap_or_else(|| die("profile must be balanced, performance or battery"))
            }
            "--start-thresh" => {
                cli.start = Some(
                    val()
                        .parse::<u8>()
                        .ok()
                        .filter(|v| *v < 100)
                        .unwrap_or_else(|| die("--start-thresh must be 0-99")),
                )
            }
            "--stop-thresh" => {
                cli.stop = Some(
                    val()
                        .parse::<u8>()
                        .ok()
                        .filter(|v| (1..=100).contains(v))
                        .unwrap_or_else(|| die("--stop-thresh must be 1-100")),
                )
            }
            "-o" | "--output" => cli.output = Some(val()),
            "--install" => cli.install = true,
            "--uninstall" => cli.uninstall = true,
            "--apply" => cli.apply = true,
            "--diff" => cli.diff = true,
            "--against" => cli.against = Some(val()),
            "-r" | "--report" => cli.report = true,
            "-v" | "--verbose" => cli.verbose = true,
            "--terse" => cli.terse = true,
            "--format" => {
                cli.format = Format::parse(&val())
                    .unwrap_or_else(|| die("--format must be conf, nix or json"))
            }
            "--demo" => cli.demo = true,
            "--tlp-version" => cli.tlp_version = Some(parse_tlp_version(&val())),
            "--only" => cli.only.extend(parse_sections(&val())),
            "--skip" => cli.skip.extend(parse_sections(&val())),
            "--sysroot" => cli.sysroot = Some(val()),
            other => {
                usage();
                die(&format!("unknown option '{}'", other));
            }
        }
    }
    if cli.sysroot.is_none() {
        cli.sysroot = std::env::var("AUTO_TLP_SYSROOT")
            .ok()
            .filter(|s| !s.is_empty());
    }
    cli
}

/// Reject option combinations that make no sense, with a message that says why.
fn validate(c: &Cli) {
    let writes = c.install || c.uninstall;
    if c.install && c.uninstall {
        die("--install and --uninstall cannot be combined");
    }
    if c.apply && !writes {
        die("--apply only makes sense together with --install or --uninstall");
    }
    if c.demo && writes {
        die("--demo uses a made-up machine and cannot be combined with --install or --uninstall");
    }
    if c.sysroot.is_some() && writes {
        die(
            "--sysroot scans a directory tree and cannot be combined with --install or --uninstall",
        );
    }
    if c.against.is_some() && !c.diff {
        die("--against only makes sense together with --diff");
    }
    if c.format != Format::Conf && (writes || c.diff) {
        die("--install, --uninstall and --diff only work with --format conf (the TLP config format)");
    }
    if c.diff && (writes || c.output.is_some() || c.report) {
        die("--diff only compares; do not combine it with --install, --uninstall, --output or --report");
    }
}

fn is_root() -> bool {
    util::run("id", &["-u"]).as_deref() == Some("0")
}

/// `tlp start`, reporting (not failing) if it does not work.
fn apply_tlp() {
    if !util::command_exists("tlp") {
        eprintln!("note: 'tlp' was not found. Install TLP, then run: sudo tlp start");
        return;
    }
    match std::process::Command::new("tlp").arg("start").status() {
        Ok(s) if s.success() => eprintln!("Applied: `tlp start` succeeded."),
        Ok(s) => eprintln!("warning: `tlp start` exited with {}", s),
        Err(e) => eprintln!("warning: could not run `tlp start`: {}", e),
    }
}

fn print_notes(warnings: &[String]) {
    if !warnings.is_empty() {
        eprintln!("\nNotes:");
        for w in warnings {
            eprintln!("  * {}", w);
        }
    }
}

fn uninstall(apply: bool) {
    if !is_root() {
        die("--uninstall needs root: run with sudo");
    }
    if !std::path::Path::new(DROPIN).exists() {
        eprintln!("Nothing to remove: {} does not exist.", DROPIN);
        return;
    }
    std::fs::remove_file(DROPIN)
        .unwrap_or_else(|e| die(&format!("cannot remove {}: {}", DROPIN, e)));
    eprintln!("Removed {}", DROPIN);
    if apply {
        apply_tlp();
    } else {
        eprintln!("\nApply now with: sudo tlp start");
    }
}

fn main() {
    let cli = parse_args();
    validate(&cli);

    if cli.uninstall {
        return uninstall(cli.apply);
    }

    if let Some(dir) = &cli.sysroot {
        let p = PathBuf::from(dir);
        if !p.is_dir() {
            die(&format!("--sysroot: '{}' is not a directory", dir));
        }
        util::set_root(p);
    }
    if !cli.demo && !util::has_root() && !std::path::Path::new("/sys").exists() {
        die("this tool needs Linux (/sys not found)");
    }

    let mut hw = hw::detect();
    if cli.demo {
        // Keep the real TLP detection (so output matches the TLP you have), fake everything else.
        let tlp = std::mem::take(&mut hw.tlp);
        hw = demo::thinkpad();
        if tlp.installed {
            hw.tlp = tlp;
        }
    }
    if let Some((maj, min, text)) = cli.tlp_version.clone() {
        hw.tlp.installed = true;
        hw.tlp.version = Some((maj, min));
        hw.tlp.version_str = Some(text);
    }
    if cli.report {
        report::print(&hw, cli.verbose);
        return;
    }

    let g = gen::generate(
        &hw,
        &Options {
            profile: cli.profile,
            start: cli.start,
            stop: cli.stop,
            terse: cli.terse,
            only: cli.only.clone(),
            skip: cli.skip.clone(),
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

    if cli.diff {
        let (old_text, label) = match &cli.against {
            Some(f) => (
                util::read(f).unwrap_or_else(|| die(&format!("cannot read '{}'", f))),
                f.clone(),
            ),
            None => match util::read(DROPIN) {
                Some(t) => (t, format!("installed ({DROPIN})")),
                None => {
                    eprintln!("note: {DROPIN} does not exist, so every setting counts as new.");
                    (String::new(), format!("nothing installed ({DROPIN})"))
                }
            },
        };
        let changes = diff::compare(&diff::parse(&old_text), &diff::parse(&g.text));
        print!("{}", diff::render(&changes, &label));
        print_notes(&warnings);
        exit(if changes.is_empty() { 0 } else { 1 });
    }

    let rendered = format::render(cli.format, &g.text);
    if cli.install {
        if !is_root() {
            die("--install needs root: run with sudo");
        }
        let _ = std::fs::create_dir_all("/etc/tlp.d");
        if std::path::Path::new(DROPIN).exists() {
            let _ = std::fs::copy(DROPIN, format!("{}.bak", DROPIN));
        }
        std::fs::write(DROPIN, &rendered)
            .unwrap_or_else(|e| die(&format!("cannot write {}: {}", DROPIN, e)));
        eprintln!("Wrote {}", DROPIN);
    } else if let Some(path) = &cli.output {
        if path == "-" {
            print!("{}", rendered);
        } else {
            std::fs::write(path, &rendered)
                .unwrap_or_else(|e| die(&format!("cannot write {}: {}", path, e)));
            eprintln!("Wrote {}", path);
        }
    } else {
        print!("{}", rendered);
    }

    print_notes(&warnings);
    if cli.install {
        if cli.apply {
            apply_tlp();
        } else {
            eprintln!("\nApply now with: sudo tlp start    Verify with: sudo tlp-stat -c  and  sudo tlp-stat -s");
        }
    }
}
