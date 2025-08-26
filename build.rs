#[cfg(feature = "windows")]
fn main() {
    let mut res = winres::WindowsResource::new();
    res.set_icon("misc/Tagspeak.ico");
    res.set_manifest_file("misc/app.manifest");
    res.compile().expect("Failed to embed resources");
}

#[cfg(not(feature = "windows"))]
fn main() {}
