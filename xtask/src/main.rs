#[macro_use]
extern crate clap;

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
    let matches = clap_app!(xtask =>
        (version: "0.1.0")
        (author: "The Hypatia Authors")
        (about: "Build support for the Hypatia system")
        (@subcommand build =>
            (about: "Builds Hypatia")
            (@arg release: conflicts_with[debug] --release "Build release version")
            (@arg debug: conflicts_with[release] --debug "Build debug version (default)")
        )
        (@subcommand dist =>
            (about: "Builds multibootable Hypatia images")
            (@arg release: conflicts_with[debug] --release "Build a release version")
            (@arg debug: conflicts_with[release] --debug "Build a debug version")
        )
        (@subcommand test =>
            (about: "Builds multibootable Hypatia images")
            (@arg release: conflicts_with[debug] --release "Build a release version")
            (@arg debug: conflicts_with[release] --debug "Build a debug version")
        )
        (@subcommand qemu =>
            (about: "Boot Theon under QEMU")
            (@arg release: conflicts_with[debug] --release "Build a release version")
            (@arg debug: conflicts_with[release] --debug "Build a debug version")
        )
        (@subcommand clean =>
            (about: "Cargo clean")
        )
    )
    .get_matches();
    if let Err(e) = match matches.subcommand() {
        ("build", Some(m)) => build(build_type(&m)),
        ("dist", Some(m)) => dist(build_type(&m)),
        ("test", Some(m)) => test(build_type(&m)),
        ("qemu", Some(m)) => qemu(build_type(&m)),
        ("clean", _) => clean(),
        _ => Err("bad subcommand".into()),
    } {
        eprintln!("{}", e);
        process::exit(1);
    }
}

fn build_type(matches: &clap::ArgMatches) -> Build {
    if matches.is_present("release") {
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
    env_or("OBJCOPY", "llvm-objcopy")
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
        .status()?;
    if !status.success() {
        return Err("objcopy failed".into());
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

fn qemu(profile: Build) -> Result<()> {
    dist(profile)?;
    let status = Command::new(qemu_system_x86_64())
        .arg("-nographic")
        .arg("-cpu")
        .arg("kvm64,+rdtscp,+pdpe1gb,+fsgsbase,+x2apic")
        .arg("-kernel")
        .arg(format!("target/{}/{}/theon.elf32", target(), profile.dir()))
        .arg("-initrd")
        .arg("bin.a")
        .current_dir(workspace())
        .status()?;
    if !status.success() {
        return Err("qemu failed".into());
    }
    Ok(())
}

fn clean() -> Result<()> {
    let status = Command::new(cargo())
        .current_dir(workspace())
        .arg("clean")
        .status()?;
    if !status.success() {
        return Err("clean failed".into());
    }
    Ok(())
}

fn workspace() -> PathBuf {
    Path::new(&env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(1)
        .unwrap()
        .to_path_buf()
}
