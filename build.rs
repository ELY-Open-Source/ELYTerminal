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
    let full =
        fs::canonicalize(&from).unwrap_or_else(|e| panic!("build: {} unread: {e}", from.display()));
    // Tests copy it beside themselves.
    println!("cargo:rustc-env=ELY_CONPTY={}", full.display());
    let out = PathBuf::from(env::var("OUT_DIR").expect("build: cargo names no OUT_DIR"));
    let named = |n: usize| out.ancestors().nth(n).and_then(|dir| dir.file_name());
    let dirs = if named(2) == Some("build".as_ref()) {
        // Cargo's own: PROFILE/build/PACKAGE-HASH/out.
        let profile = out.ancestors().nth(3).expect("a profile folder");
        vec![
            profile.to_path_buf(),
            profile.join("deps"),
            profile.join("examples"),
        ]
    } else if named(3) == Some("build".as_ref()) {
        // A shared one: PROFILE/build/PACKAGE/HASH/out.
        let profile = out.ancestors().nth(4).expect("a profile folder");
        let target = env::var("TARGET").expect("build: cargo names no TARGET");
        let mut place = PathBuf::new();
        if profile.parent().and_then(|dir| dir.file_name()) == Some(target.as_ref()) {
            place.push(&target);
        }
        place.push(profile.file_name().expect("a profile name"));
        // This cargo always uses ./target.
        let manifest = env::var("CARGO_MANIFEST_DIR").expect("build: no manifest folder");
        let profile = PathBuf::from(manifest).join("target").join(place);
        vec![profile.clone(), profile.join("examples")]
    } else {
        panic!(
            "build: OUT_DIR {} lies under no PROFILE/build",
            out.display()
        )
    };
    for dir in dirs {
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
