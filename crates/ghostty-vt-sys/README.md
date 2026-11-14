# termlevel-ghostty-vt-sys

Raw FFI generated from official **libghostty-vt 1.0.1** headers, linked to its
official precompiled macOS static library. Consumers do not need Zig,
bindgen, libclang or a Ghostty source checkout.

## Platforms and build behavior

Supported targets: `aarch64-apple-darwin`, `x86_64-apple-darwin`. A macOS host
with Xcode command line tools is required. Other targets fail explicitly;
there is no implicit fallback to a different ABI or a source build.

On the first build, `build.rs` downloads the official XCFramework with the
system HTTPS `curl`, verifies its pinned SHA256, and caches it in Cargo's
`OUT_DIR`. It extracts only the expected macOS library member, uses `lipo`
to select the **target** architecture and refreshes the archive index with
`ranlib`. It then links statically. No prebuilt binary is included in the
crate source package. Cache lifetime follows Cargo's build directory.

| Variable | Meaning |
| --- | --- |
| `GHOSTTY_VT_ARCHIVE` | Local path to the original official XCFramework **zip**, still checksum-verified |
| `GHOSTTY_VT_OFFLINE=1` | Fail instead of downloading if no cached archive exists |
| `CARGO_NET_OFFLINE=true` | Also disables this crate's download |

Cargo's `--offline` flag alone controls Cargo registry access, not arbitrary
build scripts. Set one of the offline environment variables for a strict
offline build. With an existing archive:

```sh
GHOSTTY_VT_ARCHIVE=/path/to/ghostty-vt-1.0.1.xcframework.zip \
GHOSTTY_VT_OFFLINE=1 cargo build -p termlevel-ghostty-vt
```

If a cached zip is corrupt, the build fails; remove it to download again.
No environment override bypasses artifact integrity verification.

## Provenance

- [Official installation instructions](https://lib.ghostty.org/vt/installing.html)
- Artifact: `https://release.files.ghostty.org/libghostty-vt/1.0.1/ghostty-vt-1.0.1.xcframework.zip`
- SHA256: `c0f9631277f726154c8f521954d17cdc41aac4626fc0df8cf925a19ee58e829b`
- Headers: copied unchanged from the artifact's
  `ghostty-vt.xcframework/macos-arm64_x86_64/Headers/ghostty/`.
- [Official checksum manifest](https://release.files.ghostty.org/libghostty-vt/1.0.1/SHA256SUMS)
- Ghostty's MIT license is included as `LICENSE-GHOSTTY`.

`src/bindings.rs` is generated with **bindgen-cli 0.72.1**. `src/helpers.rs`
translates the three inline mode helpers and C macros bindgen cannot emit.
Maintainers can regenerate both from the checked-in headers:

```sh
python3 crates/ghostty-vt-sys/regenerate.py
```

Regeneration needs bindgen/libclang and Python; normal builds do not. The
native test compares key generated Rust layouts to these headers using Clang.

Use the safe [termlevel-ghostty-vt](../ghostty-vt/README.md) wrapper where possible.
The raw API requires callers to honor allocation, tagged-union, thread,
callback and borrowed-buffer contracts documented in the bundled C headers.
