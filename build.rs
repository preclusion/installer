fn main() {
    // Locate kadr.exe and copy it (or a stub) into OUT_DIR so include_bytes! always works.
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let dest = format!("{out_dir}/kadr_embedded.exe");

    let source = std::env::var("KADR_EXE_SRC")
        .unwrap_or_else(|_| "../target/release/kadr.exe".to_owned());

    if std::path::Path::new(&source).exists() {
        std::fs::copy(&source, &dest).expect("copy kadr.exe");
        println!("cargo:rerun-if-changed={source}");
    } else {
        std::fs::write(&dest, b"KADR_STUB_NOT_BUILT").expect("write stub");
        println!("cargo:warning=kadr.exe not found at `{source}`. Run: cargo build --release -p kadr first.");
    }

    println!("cargo:rerun-if-env-changed=KADR_EXE_SRC");
    println!("cargo:rustc-env=KADR_EXE_PATH={dest}");

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
