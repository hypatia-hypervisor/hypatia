// Copyright 2021  The Hypatia Authors
// All rights reserved
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

#![forbid(absolute_paths_not_starting_with_crate)]
#![forbid(elided_lifetimes_in_paths)]
#![forbid(unsafe_op_in_unsafe_fn)]

use clap::Parser;
use std::{
    env,
    path::{Path, PathBuf},
    process,
};

type DynError = Box<dyn std::error::Error>;
type Result<T> = std::result::Result<T, DynError>;

#[derive(Parser)]
#[command(
    name = "hypatia",
    author = "The Hypatia Authors",
    version = "0.1.0",
    about = "Build support for the Hypatia hypervisor"
)]
struct XTask {
    #[clap(subcommand)]
    cmd: Command,
}

#[derive(Parser)]
enum Command {
    /// Builds Hypatia
    Build {
        #[clap(flatten)]
        profile: ProfileArg,
        #[clap(flatten)]
        locked: Locked,
    },
    /// Builds multibootable Hypatia images
    Dist {
        #[clap(flatten)]
        profile: ProfileArg,
        #[clap(flatten)]
        locked: Locked,
    },
    /// Builds multibootable Hypatia images and packages them into an archive
    Archive {
        #[clap(flatten)]
        profile: ProfileArg,
        #[clap(flatten)]
        locked: Locked,
    },
    /// Runs unit tests
    Test {
        #[clap(flatten)]
        profile: ProfileArg,
        #[clap(flatten)]
        locked: Locked,
    },
    /// Runs the Clippy linter
    Lint {
        #[clap(flatten)]
        locked: Locked,
    },
    /// Boots under QEMU with KVM
    Run {
        #[clap(flatten)]
        profile: ProfileArg,
        #[clap(flatten)]
        locked: Locked,
        #[arg(long, default_value_t = 4)]
        smp: u32,
        #[arg(long, default_value_t = 2048)]
        ram: u32,
    },
    /// Expands macros
    Expand,
    /// Cleans build artifacts
    Clean,
}

/// The build profile to use, either debug or release.
/// Debug is the default.
#[derive(Clone, Copy)]
enum Profile {
    Debug,
    Release,
}

impl Profile {
    fn dir(&self) -> &str {
        match self {
            Self::Debug => "debug",
            Self::Release => "release",
        }
    }

    fn as_str(&self) -> &str {
        match self {
            Self::Debug => "",
            Self::Release => "--release",
        }
    }
}

// A workaround since Clap doesn't currently support deriving an enum.
#[derive(Clone, Parser)]
#[clap(group = clap::ArgGroup::new("profile").multiple(false))]
struct ProfileArg {
    /// Build debug version (default)
    #[clap(long, group = "profile")]
    debug: bool,
    /// Build release version
    #[clap(long, group = "profile")]
    release: bool,
}

impl From<ProfileArg> for Profile {
    fn from(p: ProfileArg) -> Profile {
        match (p.debug, p.release) {
            (false, true) => Profile::Release,
            (_, false) => Profile::Debug,
            _ => unreachable!("impossible"),
        }
    }
}

/// Whether to run Cargo with `--locked`.
#[derive(Parser)]
struct Locked {
    /// Build locked to Cargo.lock
    #[clap(long)]
    locked: bool,
}
impl Locked {
    fn as_str(&self) -> &str {
        if self.locked { "--locked" } else { "" }
    }
}

fn main() {
    let xtask = XTask::parse();
    if let Err(e) = match xtask.cmd {
        Command::Build { profile, locked } => build(profile.into(), locked),
        Command::Dist { profile, locked } => dist(profile.into(), locked),
        Command::Archive { profile, locked } => archive(profile.into(), locked),
        Command::Test { profile, locked } => test(profile.into(), locked),
        Command::Lint { locked } => lint(locked),
        Command::Run { profile, locked, smp, ram } => run(profile.into(), locked, smp, ram),
        Command::Expand => expand(),
        Command::Clean => clean(),
    } {
        eprintln!("error: {e:?}");
        process::exit(1);
    }
}

fn build(profile: Profile, locked: Locked) -> Result<()> {
    let args = format!(
        "build {profile} {locked} \
            --workspace --exclude xtask \
            -Z build-std=core,alloc \
            --target lib/{triple}.json",
        profile = profile.as_str(),
        locked = locked.as_str(),
        triple = target(),
    );
    let status = process::Command::new(cargo())
        .current_dir(workspace())
        .args(args.split_whitespace())
        .status()?;
    if !status.success() {
        return Err("build failed".into());
    }
    Ok(())
}

fn dist(profile: Profile, locked: Locked) -> Result<()> {
    build(profile, locked)?;
    let args = format!(
        "--input-target=elf64-x86-64 --output-target=elf32-i386 \
            target/{triple}/{profile}/theon \
            target/{triple}/{profile}/theon.elf32",
        triple = target(),
        profile = profile.dir(),
    );
    let status = process::Command::new(objcopy())
        .args(args.split_whitespace())
        .current_dir(workspace())
        .status()
        .map_err(|e| format!("objcopy failed. Have you installed llvm? {e}"))?;
    if !status.success() {
        return Err("objcopy failed".into());
    }
    Ok(())
}

fn archive(profile: Profile, locked: Locked) -> Result<()> {
    const BINS: &[&str] = &[
        "devices",
        "global",
        "memory",
        "monitor",
        "node",
        "scheduler",
        "supervisor",
        "system",
        "trace",
        "vcpu",
        "vm",
    ];

    dist(profile, locked)?;
    let _ = std::fs::remove_file(arname());
    let mut a = ar::Builder::new(std::fs::File::create(arname())?);
    for bin in BINS {
        let filename = workspace().join("target").join(target()).join(profile.dir()).join(bin);
        a.append_path(filename)?;
    }
    Ok(())
}

fn test(profile: Profile, locked: Locked) -> Result<()> {
    let args =
        format!("test {profile} {locked}", profile = profile.as_str(), locked = locked.as_str());
    let status = process::Command::new(cargo())
        .current_dir(workspace())
        .args(args.split_whitespace())
        .status()?;
    if !status.success() {
        return Err("test failed".into());
    }
    Ok(())
}

fn lint(locked: Locked) -> Result<()> {
    let args = format!("clippy {locked}", locked = locked.as_str());
    let status = process::Command::new(cargo())
        .current_dir(workspace())
        .args(args.split_whitespace())
        .status()?;
    if !status.success() {
        return Err("lint failed".into());
    }
    Ok(())
}

fn run(profile: Profile, locked: Locked, smp: u32, ram: u32) -> Result<()> {
    archive(profile, locked)?;
    let args = format!(
        "-nographic \
            -accel kvm \
            -cpu kvm64,+rdtscp,+pdpe1gb,+fsgsbase,+x2apic \
            -machine q35 \
            -smp {smp} \
            -m {ram} \
            -kernel target/{triple}/{profile}/theon.elf32 \
            -initrd {archive}",
        triple = target(),
        profile = profile.dir(),
        archive = arname().display(),
    );
    let status = process::Command::new(qemu_system_x86_64())
        .args(args.split_whitespace())
        .current_dir(workspace())
        .status()?;
    if !status.success() {
        return Err("qemu failed".into());
    }
    Ok(())
}

fn expand() -> Result<()> {
    let status = process::Command::new(cargo())
        .current_dir(workspace())
        .arg("rustc")
        .arg("--")
        .arg("-Zunpretty=expanded")
        .status()?;
    if !status.success() {
        return Err("expand failed".into());
    }
    Ok(())
}

fn clean() -> Result<()> {
    let status = process::Command::new(cargo()).current_dir(workspace()).arg("clean").status()?;
    if !status.success() {
        return Err("clean failed".into());
    }
    Ok(())
}

fn env_or(var: &str, default: &str) -> String {
    let default = default.to_string();
    env::var(var).unwrap_or(default)
}

fn cargo() -> String {
    env_or("CARGO", "cargo")
}

fn objcopy() -> String {
    let llvm_objcopy = {
        let toolchain = env_or("RUSTUP_TOOLCHAIN", "x86_64-unknown-none");
        let pos = toolchain.find('-').map(|p| p + 1).unwrap_or(0);
        let host = toolchain[pos..].to_string();
        let home = env_or("RUSTUP_HOME", "");
        let mut path = PathBuf::from(home);
        path.push("toolchains");
        path.push(toolchain);
        path.push("lib");
        path.push("rustlib");
        path.push(host);
        path.push("bin");
        path.push("llvm-objcopy");
        if path.exists() {
            path.into_os_string().into_string().unwrap()
        } else {
            "llvm-objcopy".into()
        }
    };
    env_or("OBJCOPY", &llvm_objcopy)
}

fn qemu_system_x86_64() -> String {
    env_or("QEMU", "qemu-system-x86_64")
}

fn target() -> String {
    env_or("TARGET", "x86_64-unknown-none-elf")
}

fn workspace() -> PathBuf {
    Path::new(&env!("CARGO_MANIFEST_DIR")).ancestors().nth(1).unwrap().to_path_buf()
}

fn arname() -> PathBuf {
    workspace().join("target").join("bin.a")
}
