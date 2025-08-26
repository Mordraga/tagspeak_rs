// build.rs (root)
fn main() {
    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();
        // engine icon:
        res.set_icon("misc/Tagspeak.ico");
        // manifest for Common Controls v6:
        res.set_manifest_file("misc/app.manifest");
        res.compile().expect("Failed to embed resources");
    }
}
