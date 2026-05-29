fn main() {
    // ── kadr version (read from kadr/Cargo.toml) ──────────────────────────────
    let kadr_version = std::fs::read_to_string("../kadr/Cargo.toml")
        .ok()
        .and_then(|t| {
            t.lines()
                .find(|l| l.trim_start().starts_with("version") && l.contains('"'))
                .and_then(|l| l.split('"').nth(1))
                .map(|s| s.to_owned())
        })
        .unwrap_or_else(|| "unknown".to_owned());
    println!("cargo:rustc-env=KADR_VERSION={kadr_version}");
    println!("cargo:rerun-if-changed=../kadr/Cargo.toml");

    // Windows manifest + version info
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winres::WindowsResource::new();
        res.set("ProductName", "Kadr Installer");
        res.set("FileDescription", "Kadr Image Viewer Installer");
        res.set("LegalCopyright", "");
        res.set_manifest(r#"
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    <security>
      <requestedPrivileges>
        <requestedExecutionLevel level="asInvoker" uiAccess="false"/>
      </requestedPrivileges>
    </security>
  </trustInfo>
  <compatibility xmlns="urn:schemas-microsoft-com:compatibility.v1">
    <application>
      <supportedOS Id="{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}"/>
    </application>
  </compatibility>
</assembly>
"#);
        let _ = res.compile();
    }
}
