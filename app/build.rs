//! Gives the Windows exe its icon and version info (what Explorer, the taskbar and the installer show).

fn main() {
    println!("cargo:rerun-if-changed=icons/icon.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let mut res = winresource::WindowsResource::new();
    res.set_icon("icons/icon.ico")
        .set("ProductName", "Logbook")
        .set("FileDescription", "Logbook")
        .set("CompanyName", "Etienne de Lange")
        .set("OriginalFilename", "logbook.exe")
        .set("InternalName", "logbook");
    if let Err(e) = res.compile() {
        // A missing resource compiler shouldn't stop a build: the exe just has the default icon.
        println!("cargo:warning=couldn't embed the Windows icon and version info: {e}");
    }
}
