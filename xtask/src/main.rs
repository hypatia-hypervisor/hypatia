// Copyright 2021  The Hypatia Authors
// All rights reserved
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

#![forbid(absolute_paths_not_starting_with_crate)]
#![forbid(elided_lifetimes_in_paths)]
#![forbid(unsafe_op_in_unsafe_fn)]

use std::{
    env,
    path::{Path, PathBuf},
    process::{self, Command},
};

type DynError = Box<dyn std::error::Error>;
type Result<T> = std::result::Result<T, DynError>;

#[derive(Clone, Copy)]
enum Build {
    Debug,
    Release,
}
impl Build {
    fn dir(self) -> &'static str {
        match self {
            Self::Debug => "debug",
            Self::Release => "release",
        }
    }

    fn add_build_arg(self, cmd: &mut Command) {
        if let Self::Release = self {
            cmd.arg("--release");
        }
    }
}

fn main() {
    let matches = clap::Command::new("xtask")
        .version("0.1.0")
        .author("The Hypatia Authors")
        .about("Build support for the Hypatia system")
        .subcommand(clap::Command::new("build").about("Builds Hypatia").args(&[
            clap::arg!(--release "Build release version").conflicts_with("debug"),
            clap::arg!(--debug "Build debug version (default)").conflicts_with("release"),
        ]))
        .subcommand(clap::Command::new("dist").about("Builds multibootable Hypatia images").args(
            &[
                clap::arg!(--release "Build a release version").conflicts_with("debug"),
                clap::arg!(--debug "Build a debug version").conflicts_with("release"),
            ],
        ))
        .subcommand(
            clap::Command::new("archive")
                .about("Builds multibootable Hypatia images and packages them into an archive")
                .args(&[
                    clap::arg!(--release "Build a release version").conflicts_with("debug"),
                    clap::arg!(--debug "Build a debug version").conflicts_with("release"),
                ]),
        )
        .subcommand(clap::Command::new("test").about("Builds multibootable Hypatia images").args(
            &[
                clap::arg!(--release "Build a release version").conflicts_with("debug"),
                clap::arg!(--debug "Build a debug version").conflicts_with("release"),
            ],
        ))
        .subcommand(clap::Command::new("lint").about("Cargo clippy"))
        .subcommand(clap::Command::new("qemu").about("Boot Theon under QEMU").args(&[
            clap::arg!(--release "Build a release version").conflicts_with("debug"),
            clap::arg!(--debug "Build a debug version").conflicts_with("release"),
        ]))
        .subcommand(clap::Command::new("qemukvm").about("Boot Theon under QEMU with KVM").args(&[
            clap::arg!(--release "Build a release version").conflicts_with("debug"),
            clap::arg!(--debug "Build a debug version").conflicts_with("release"),
        ]))
        .subcommand(clap::Command::new("clean").about("Cargo clean"))
        .get_matches();
    if let Err(e) = match matches.subcommand() {
        Some(("build", m)) => build(build_type(m)),
        Some(("dist", m)) => dist(build_type(m)),
        Some(("archive", m)) => archive(build_type(m)),
        Some(("test", m)) => test(build_type(m)),
        Some(("lint", _)) => lint(),
        Some(("qemu", m)) => qemu(build_type(m)),
        Some(("qemukvm", m)) => qemukvm(build_type(m)),
        Some(("clean", _)) => clean(),
        _ => Err("bad subcommand".into()),
    } {
        eprintln!("{}", e);
        process::exit(1);
    }
}

fn build_type(matches: &clap::ArgMatches) -> Build {
    if matches.contains_id("release") {
        return Build::Release;
    }
    Build::Debug
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

fn build(profile: Build) -> Result<()> {
    let mut cmd = Command::new(cargo());
    cmd.current_dir(workspace());
    cmd.arg("build");
    cmd.arg("--workspace").arg("--exclude").arg("xtask");
    cmd.arg("-Z").arg("build-std=core,alloc");
    cmd.arg("--target").arg(format!("lib/{}.json", target()));
    profile.add_build_arg(&mut cmd);
    let status = cmd.status()?;
    if !status.success() {
        return Err("build failed".into());
    }
    Ok(())
}

fn dist(profile: Build) -> Result<()> {
    build(profile)?;
    let status = Command::new(objcopy())
        .arg("--input-target=elf64-x86-64")
        .arg("--output-target=elf32-i386")
        .arg(format!("target/{}/{}/theon", target(), profile.dir()))
        .arg(format!("target/{}/{}/theon.elf32", target(), profile.dir()))
        .current_dir(workspace())
        .status()
        .map_err(|e| format!("objcopy failed. Have you installed llvm? {e}"))?;
    if !status.success() {
        return Err("objcopy failed".into());
    }
    Ok(())
}

const BINS: &[&str] = &[
    "devices",
    "global",
    "memory",
    "monitor",
    "scheduler",
    "supervisor",
    "system",
    "trace",
    "vcpu",
    "vm",
];

fn archive(profile: Build) -> Result<()> {
    dist(profile)?;
    let _ = std::fs::remove_file(arname());
    let mut a = ar::Builder::new(std::fs::File::create(arname())?);
    for bin in BINS {
        let filename = workspace().join("target").join(target()).join(profile.dir()).join(bin);
        a.append_path(filename)?;
    }
    Ok(())
}

fn test(profile: Build) -> Result<()> {
    let mut cmd = Command::new(cargo());
    cmd.current_dir(workspace());
    cmd.arg("test");
    profile.add_build_arg(&mut cmd);
    let status = cmd.status()?;
    if !status.success() {
        return Err("test failed".into());
    }
    Ok(())
}

fn lint() -> Result<()> {
    let status = Command::new(cargo()).current_dir(workspace()).arg("clippy").status()?;
    if !status.success() {
        return Err("lint failed".into());
    }
    Ok(())
}

fn qemu(profile: Build) -> Result<()> {
    archive(profile)?;
    let status = Command::new(qemu_system_x86_64())
        .arg("-nographic")
        .arg("-cpu")
        .arg("kvm64,+rdtscp,+pdpe1gb,+fsgsbase,+x2apic")
        .arg("-m")
        .arg("2048")
        .arg("-kernel")
        .arg(format!("target/{}/{}/theon.elf32", target(), profile.dir()))
        .arg("-initrd")
        .arg(arname())
        .current_dir(workspace())
        .status()
        .map_err(|e| format!("qemu failed. Have you installed qemu-system-x86? {e}"))?;
    if !status.success() {
        return Err("qemu failed".into());
    }
    Ok(())
}

fn qemukvm(profile: Build) -> Result<()> {
    archive(profile)?;
    let status = Command::new(qemu_system_x86_64())
        .arg("-nographic")
        .arg("-accel")
        .arg("kvm")
        .arg("-cpu")
        .arg("kvm64,+rdtscp,+pdpe1gb,+fsgsbase,+x2apic")
        .arg("-m")
        .arg("2048")
        .arg("-kernel")
        .arg(format!("target/{}/{}/theon.elf32", target(), profile.dir()))
        .arg("-initrd")
        .arg(arname())
        .current_dir(workspace())
        .status()?;
    if !status.success() {
        return Err("qemu failed".into());
    }
    Ok(())
}

fn clean() -> Result<()> {
    let status = Command::new(cargo()).current_dir(workspace()).arg("clean").status()?;
    if !status.success() {
        return Err("clean failed".into());
    }
    Ok(())
}

fn workspace() -> PathBuf {
    Path::new(&env!("CARGO_MANIFEST_DIR")).ancestors().nth(1).unwrap().to_path_buf()
}

fn arname() -> PathBuf {
    workspace().join("target").join("bin.a")
}
