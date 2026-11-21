fn main() {
    // Native Tauri mock tests also import the Common Controls v6 APIs. Embed
    // the activation context in every executable target, including lib tests.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg=/MANIFESTDEPENDENCY:type='win32' name='Microsoft.Windows.Common-Controls' version='6.0.0.0' processorArchitecture='*' publicKeyToken='6595b64144ccf1df' language='*'");
    }
    // The linker above embeds the common activation context for every target;
    // omit the duplicate manifest from the icon/version resource library.
    tauri_build::try_build(tauri_build::Attributes::new().windows_attributes(
        tauri_build::WindowsAttributes::new_without_app_manifest(),
    )).expect("Tauri build configuration failed")
}
