//! Attach Windows resources (icon, version, company) to the executable.
//!
//! A failure here is never fatal: the resource compiler is not present on
//! every Windows box, and the binary is perfectly usable without it.

fn main() {
    println!("cargo:rerun-if-changed=../../assets/icon.ico");

    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("../../assets/icon.ico");
        res.set("ProductName", "no-watermark");
        res.set("FileDescription", env!("CARGO_PKG_DESCRIPTION"));
        res.set("CompanyName", "Giacomo Giordano");
        res.set(
            "LegalCopyright",
            "Copyright (c) 2026 Giacomo Giordano - MIT",
        );
        if let Err(e) = res.compile() {
            println!("cargo:warning=windows resources not embedded: {e}");
        }
    }
}
