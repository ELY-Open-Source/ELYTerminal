//! Windows: the icon as resource 1, ConPTY beside each executable.

use std::{env, fs, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=assets/icons");
    println!("cargo:rerun-if-changed=vendor/conpty");
    embed_resource::compile(
        "assets/icons/elyterminal.rc",
        embed_resource::ParamsIncludeDirs(["assets/icons"]),
    )
    .manifest_required()
    .expect("build: the icon did not go into the Windows executable");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        conpty();
    }
}

/// Copies the target's `conpty.dll` and `OpenConsole.exe` where its executables land.
fn conpty() {
    let arch = env::var("CARGO_CFG_TARGET_ARCH").expect("build: cargo names no target arch");
    let from = PathBuf::from("vendor/conpty").join(&arch);
    let out = PathBuf::from(env::var("OUT_DIR").expect("build: cargo names no OUT_DIR"));
    let profile = out
        .ancestors()
        .nth(3)
        .filter(|_| out.ancestors().nth(2).and_then(|b| b.file_name()) == Some("build".as_ref()))
        .unwrap_or_else(|| {
            panic!(
                "build: OUT_DIR {} is not under target/PROFILE/build",
                out.display()
            )
        });
    for dir in [
        profile.to_path_buf(),
        profile.join("deps"),
        profile.join("examples"),
    ] {
        fs::create_dir_all(&dir)
            .unwrap_or_else(|e| panic!("build: {} not made: {e}", dir.display()));
        for file in ["conpty.dll", "OpenConsole.exe"] {
            fs::copy(from.join(file), dir.join(file)).unwrap_or_else(|e| {
                panic!(
                    "build: {file} for {arch} not copied to {}: {e}",
                    dir.display()
                )
            });
        }
    }
}
