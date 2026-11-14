use std::{fs, process::Command};
use termlevel_ghostty_vt_sys::*;

/// Check Rust's generated layout against the actual pinned C headers with an
/// independent compiler, including the sized structs used by the safe wrapper.
#[test]
fn generated_layout_matches_c_headers() {
    let directory =
        std::env::temp_dir().join(format!("termlevel-ghostty-abi-{}", std::process::id()));
    fs::create_dir_all(&directory).unwrap();
    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    let _cleanup = Cleanup(directory.clone());
    let source = directory.join("abi.c");
    let executable = directory.join("abi");
    let types = [
        (
            "GhosttyString",
            size_of::<GhosttyString>(),
            align_of::<GhosttyString>(),
        ),
        (
            "GhosttyBuffer",
            size_of::<GhosttyBuffer>(),
            align_of::<GhosttyBuffer>(),
        ),
        (
            "GhosttyStyle",
            size_of::<GhosttyStyle>(),
            align_of::<GhosttyStyle>(),
        ),
        (
            "GhosttyRenderStateCursor",
            size_of::<GhosttyRenderStateCursor>(),
            align_of::<GhosttyRenderStateCursor>(),
        ),
        (
            "GhosttyRenderStateColors",
            size_of::<GhosttyRenderStateColors>(),
            align_of::<GhosttyRenderStateColors>(),
        ),
        (
            "GhosttyMouseEncoderSize",
            size_of::<GhosttyMouseEncoderSize>(),
            align_of::<GhosttyMouseEncoderSize>(),
        ),
        (
            "GhosttyTerminalModeConfig",
            size_of::<GhosttyTerminalModeConfig>(),
            align_of::<GhosttyTerminalModeConfig>(),
        ),
    ];
    let mut code = String::from("#include <ghostty/vt.h>\n#include <stdio.h>\nint main(void) {\n");
    for (name, _, _) in types {
        code.push_str(&format!(
            "printf(\"%zu %zu \", sizeof({name}), _Alignof({name}));\n"
        ));
    }
    code.push_str("printf(\"%u %u %u\\n\", GHOSTTY_STYLE_FLAG_BOLD, GHOSTTY_MODE_FOCUS_EVENT, ghostty_mode_new(4, true));\nreturn 0; }\n");
    fs::write(&source, code).unwrap();
    let compiled = Command::new("/usr/bin/xcrun")
        .args(["clang", "-std=c11", "-Werror", "-I"])
        .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/include"))
        .arg(&source)
        .arg("-o")
        .arg(&executable)
        .output()
        .unwrap();
    assert!(
        compiled.status.success(),
        "{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let output = Command::new(executable).output().unwrap();
    assert!(output.status.success());
    let actual: Vec<usize> = String::from_utf8(output.stdout)
        .unwrap()
        .split_whitespace()
        .map(|value| value.parse().unwrap())
        .collect();
    let mut expected: Vec<usize> = types
        .into_iter()
        .flat_map(|(_, size, alignment)| [size, alignment])
        .collect();
    expected.extend([
        GHOSTTY_STYLE_FLAG_BOLD as usize,
        GHOSTTY_MODE_FOCUS_EVENT as usize,
        ghostty_mode_new(4, true) as usize,
    ]);
    assert_eq!(actual, expected);
}
