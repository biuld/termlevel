use sha2::{Digest, Sha256};
use std::{
    env, fs,
    io::{self, Read},
    path::{Path, PathBuf},
    process::Command,
};

const ARCHIVE: &str = "ghostty-vt-1.0.1.xcframework.zip";
const URL: &str =
    "https://release.files.ghostty.org/libghostty-vt/1.0.1/ghostty-vt-1.0.1.xcframework.zip";
const SHA256: &str = "c0f9631277f726154c8f521954d17cdc41aac4626fc0df8cf925a19ee58e829b";
const MEMBER: &str = "ghostty-vt.xcframework/macos-arm64_x86_64/libghostty-vt.a";

fn run(command: &mut Command) {
    let status = command
        .status()
        .unwrap_or_else(|e| panic!("failed to run {command:?}: {e}"));
    assert!(status.success(), "command failed: {command:?}");
}

fn digest(path: &Path) -> io::Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hash = Sha256::new();
    let mut buf = [0; 64 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hash.update(&buf[..n]);
    }
    Ok(format!("{:x}", hash.finalize()))
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=GHOSTTY_VT_ARCHIVE");
    println!("cargo:rerun-if-env-changed=GHOSTTY_VT_OFFLINE");
    println!("cargo:rerun-if-env-changed=CARGO_NET_OFFLINE");
    let target = env::var("TARGET").unwrap();
    let arch = match target.as_str() {
        "aarch64-apple-darwin" => "arm64",
        "x86_64-apple-darwin" => "x86_64",
        _ => panic!(
            "unsupported target {target}: prebuilt support currently covers only macOS arm64/x86_64"
        ),
    };
    assert!(
        env::var("HOST").unwrap().ends_with("apple-darwin"),
        "prebuilt extraction requires a macOS host with Xcode command line tools"
    );
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let archive = if let Some(path) = env::var_os("GHOSTTY_VT_ARCHIVE") {
        let path = PathBuf::from(path);
        println!("cargo:rerun-if-changed={}", path.display());
        path
    } else {
        let path = out.join(ARCHIVE);
        if !path.exists() {
            let offline = ["GHOSTTY_VT_OFFLINE", "CARGO_NET_OFFLINE"]
                .iter()
                .any(|key| env::var(key).is_ok_and(|value| value == "1" || value == "true"));
            assert!(
                !offline,
                "libghostty-vt archive is not cached; set GHOSTTY_VT_ARCHIVE to the official {ARCHIVE} for offline builds"
            );
            let temporary = out.join(format!("{ARCHIVE}.download"));
            run(Command::new("/usr/bin/curl")
                .args([
                    "--fail",
                    "--location",
                    "--proto",
                    "=https",
                    "--proto-redir",
                    "=https",
                    "--connect-timeout",
                    "15",
                    "--max-time",
                    "180",
                    "--retry",
                    "2",
                    "--output",
                ])
                .arg(&temporary)
                .arg(URL));
            let actual = digest(&temporary).expect("read downloaded archive");
            if actual != SHA256 {
                let _ = fs::remove_file(&temporary);
                panic!("libghostty-vt archive SHA256 mismatch: expected {SHA256}, got {actual}");
            }
            fs::rename(temporary, &path).expect("cache verified archive");
        }
        path
    };
    let actual = digest(&archive).unwrap_or_else(|e| panic!("read {}: {e}", archive.display()));
    assert_eq!(
        actual,
        SHA256,
        "libghostty-vt archive SHA256 mismatch: {}",
        archive.display()
    );

    // Extract only the pinned library member, never arbitrary zip paths.
    let universal = out.join("universal");
    let native = out.join(arch);
    fs::create_dir_all(&universal).unwrap();
    fs::create_dir_all(&native).unwrap();
    run(Command::new("/usr/bin/unzip")
        .args(["-q", "-o", "-j"])
        .arg(&archive)
        .arg(MEMBER)
        .arg("-d")
        .arg(&universal));
    let library = native.join("libghostty-vt.a");
    run(Command::new("/usr/bin/xcrun")
        .arg("lipo")
        .arg(universal.join("libghostty-vt.a"))
        .args(["-thin", arch, "-output"])
        .arg(&library));
    run(Command::new("/usr/bin/xcrun").arg("ranlib").arg(&library));
    println!("cargo:rustc-link-search=native={}", native.display());
    println!("cargo:rustc-link-lib=static=ghostty-vt");
    println!(
        "cargo:include={}/include",
        env::var("CARGO_MANIFEST_DIR").unwrap()
    );
    println!("cargo:version=1.0.1");
}
