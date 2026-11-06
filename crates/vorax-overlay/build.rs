// The .exe's resources: the icon (Explorer, the taskbar, the tray: assets/vorax.ico, drawn by dev/make_icon.py,
// SmartTorch: a flame with an S), and what it is (its name, version and maker, shown in its Properties › Details), so
// it isn't a bare program to Windows and browsers' download checks.
fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        println!("cargo:rerun-if-changed=assets/vorax.ico");
        println!("cargo:rerun-if-changed=build.rs");
        let dir = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
        let out = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
        let version = std::env::var("CARGO_PKG_VERSION").unwrap();
        let mut parts: Vec<u16> = version.split(['.', '-']).filter_map(|p| p.parse().ok()).collect();
        parts.resize(4, 0);
        let comma = parts.iter().map(|p| p.to_string()).collect::<Vec<_>>().join(",");
        let ico = dir.join("assets").join("vorax.ico").display().to_string().replace('\\', "\\\\");
        let rc = format!(
            r#"#include <winver.h>
1 ICON "{ico}"
1 VERSIONINFO
FILEVERSION {comma}
PRODUCTVERSION {comma}
FILEOS VOS_NT_WINDOWS32
FILETYPE VFT_APP
BEGIN
  BLOCK "StringFileInfo"
  BEGIN
    BLOCK "040904B0"
    BEGIN
      VALUE "CompanyName", "lauhinsnsd-dotcom"
      VALUE "FileDescription", "SmartTorch - Torchlight Infinite toolbox (reads the screen only)"
      VALUE "FileVersion", "{version}"
      VALUE "InternalName", "SmartTorch"
      VALUE "LegalCopyright", "MIT License. Based on DinoRaos/vorax-solver-rs (MIT)"
      VALUE "OriginalFilename", "SmartTorch.exe"
      VALUE "ProductName", "SmartTorch"
      VALUE "ProductVersion", "{version}"
      VALUE "Comments", "https://github.com/lauhinsnsd-dotcom/SmartTorch"
    END
  END
  BLOCK "VarFileInfo"
  BEGIN
    VALUE "Translation", 0x409, 1200
  END
END
"#
        );
        let path = out.join("smarttorch.rc");
        std::fs::write(&path, rc).unwrap();
        embed_resource::compile(&path, embed_resource::NONE).manifest_optional().unwrap();
    }
}
