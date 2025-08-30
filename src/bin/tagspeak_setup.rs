#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

#[cfg(target_os = "windows")]
use anyhow::Result;
#[cfg(target_os = "windows")]
use native_windows_derive as nwd;
#[cfg(target_os = "windows")]
use native_windows_gui as nwg;
#[cfg(target_os = "windows")]
use nwg::NativeUi;
#[cfg(target_os = "windows")]
use std::path::PathBuf;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Shell::{SHCNE_ASSOCCHANGED, SHCNF_IDLIST, SHChangeNotify};
#[cfg(target_os = "windows")]
use winreg::{RegKey, enums::*};

#[cfg(target_os = "windows")]
const PROGID: &str = "TagSpeakFile";
#[cfg(target_os = "windows")]
const DISPLAY: &str = "TagSpeak Script";

#[cfg(target_os = "windows")]
#[derive(Default, nwd::NwgUi)]
pub struct App {
    #[nwg_control(size: (560, 260), title: "TagSpeak Setup", flags: "WINDOW|VISIBLE")]
    #[nwg_events(OnWindowClose: [App::exit])]
    window: nwg::Window,

    // this is the layout control the macro items bind to
    #[nwg_layout(parent: window)]
    layout: nwg::GridLayout,

    #[nwg_control(text: "Welcome to TagSpeak Setup. This registers .tgsk with your chosen engine for your user account. No changes until you click Install.")]
    #[nwg_layout_item(layout: layout, row: 0, col: 0, col_span: 4)]
    lbl: nwg::Label,

    #[nwg_control(text: "Engine (tagspeak_rs.exe):")]
    #[nwg_layout_item(layout: layout, row: 1, col: 0, col_span: 2)]
    lbl_engine: nwg::Label,

    #[nwg_control]
    #[nwg_layout_item(layout: layout, row: 2, col: 0, col_span: 3)]
    tb_engine: nwg::TextInput,

    #[nwg_control(text: "Browse…")]
    #[nwg_events(OnButtonClick: [App::browse_engine])]
    #[nwg_layout_item(layout: layout, row: 2, col: 3)]
    btn_browse: nwg::Button,

    #[nwg_control(text: "Install")]
    #[nwg_events(OnButtonClick: [App::install])]
    #[nwg_layout_item(layout: layout, row: 3, col: 0)]
    btn_install: nwg::Button,

    #[nwg_control(text: "Uninstall")]
    #[nwg_events(OnButtonClick: [App::uninstall])]
    #[nwg_layout_item(layout: layout, row: 3, col: 1)]
    btn_uninstall: nwg::Button,

    #[nwg_control(text: "Run engine")]
    #[nwg_events(OnButtonClick: [App::run_engine])]
    #[nwg_layout_item(layout: layout, row: 3, col: 2)]
    btn_run: nwg::Button,

    #[nwg_control(text: "Close")]
    #[nwg_events(OnButtonClick: [App::exit])]
    #[nwg_layout_item(layout: layout, row: 3, col: 3)]
    btn_close: nwg::Button,

    #[nwg_control(text: "")]
    #[nwg_layout_item(layout: layout, row: 4, col: 0, col_span: 4)]
    lbl_status: nwg::Label,

    #[nwg_control(text: "Build engine")]
    #[nwg_events(OnButtonClick: [App::build_engine])]
    #[nwg_layout_item(layout: layout, row: 3, col: 2)]
    btn_build: nwg::Button,
}

#[cfg(target_os = "windows")]
impl App {
    fn post_init_cleanup(&self) {
        // Try to discover the engine via PATH if not set or missing
        let cur = self.tb_engine.text();
        if cur.is_empty() || !std::path::Path::new(&cur).exists() {
            if let Ok(p) = which::which("tagspeak_rs") {
                self.tb_engine.set_text(&p.display().to_string());
            }
        }
        if !std::path::Path::new(&self.tb_engine.text()).exists() {
            self.lbl_status
                .set_text("Select your tagspeak_rs.exe, then click Install to register .tgsk.");
        }
        // Normalize odd characters that can sneak into resources
        self.btn_browse.set_text("Browse...");
        self.update_enabled();
    }
    fn init_defaults(&self) {
        // try to auto-fill engine path (…\tagspeak_rs\target\release\tagspeak_rs.exe)
        if let Ok(mut here) = std::env::current_exe() {
            for _ in 0..2 {
                here = here.parent().unwrap().to_path_buf();
            } // up from target\release
            here.push("tagspeak_rs\\target\\release\\tagspeak_rs.exe");
            if here.exists() {
                self.tb_engine.set_text(&here.display().to_string());
            }
        }
        // nice spacing so DPI doesn’t squish stuff
        self.layout.spacing(8);
        self.layout.margin([12, 12, 12, 12]);

        self.update_enabled();

        // if engine not found, offer to build
        if !std::path::Path::new(&self.tb_engine.text()).exists() {
            self.lbl_status
                .set_text("Engine not found. Click “Build engine”.");
        }
    }

    fn update_enabled(&self) {
        let ok = std::path::Path::new(&self.tb_engine.text()).exists();
        self.btn_install.set_enabled(ok);
        self.btn_uninstall.set_enabled(true);
        self.btn_run.set_enabled(ok);
    }

    fn browse_engine(&self) {
        let mut dialog = nwg::FileDialog::default();
        if nwg::FileDialog::builder()
            .title("Select tagspeak_rs.exe")
            .action(nwg::FileDialogAction::Open)
            .filters("Executable (*.exe)|*.exe")
            .build(&mut dialog)
            .is_ok()
        {
            // Show the dialog; only proceed if user selected a file
            if dialog.run(Some(&self.window)) {
                if let Ok(os) = dialog.get_selected_item() {
                    let p = std::path::PathBuf::from(os);
                    self.tb_engine.set_text(&p.display().to_string());
                    self.update_enabled();
                }
            }
        }
    }

    fn install(&self) {
        // Ensure we have an engine; if missing, try to build if Rust toolchain is available.
        let mut exe = std::path::PathBuf::from(self.tb_engine.text());
        if !exe.exists() {
            // Try building if cargo is available
            if has_cargo() {
                self.set_busy(true, "Building TagSpeak engine… this can take a minute");
                self.build_engine();
                exe = std::path::PathBuf::from(self.tb_engine.text());
                if !exe.exists() {
                    self.set_busy(false, "Build finished but engine not found. Select it manually or try again.");
                    return;
                }
            } else {
                // Offer to open Rust installer page
                nwg::modal_info_message(
                    &self.window,
                    "Rust toolchain required",
                    "Rust (cargo) is not installed. We'll open rustup.rs in your browser. Install Rust, then return here and click Install again.",
                );
                let _ = std::process::Command::new("rundll32")
                    .args(["url.dll,FileProtocolHandler", "https://rustup.rs"]).spawn();
                return;
            }
        }

        if let Err(e) = do_install(exe) {
            nwg::modal_error_message(&self.window, "Install failed", &format!("{e:#}"));
            return;
        }
        nwg::modal_info_message(&self.window, "Done", "Associated .tgsk with TagSpeak!");
        let _ = refresh_icons();
        self.update_enabled();
    }

    fn uninstall(&self) {
        if let Err(e) = do_uninstall() {
            nwg::modal_error_message(&self.window, "Uninstall failed", &format!("{e:#}"));
            return;
        }
        nwg::modal_info_message(&self.window, "Removed", "Association for .tgsk removed.");
        let _ = refresh_icons();
        self.update_enabled();
    }

    fn run_engine(&self) {
        let path = std::path::PathBuf::from(self.tb_engine.text());
        if path.exists() {
            let _ = std::process::Command::new(path).status();
        } else {
            nwg::modal_error_message(&self.window, "Not found", "Engine path doesn’t exist.");
        }
    }

    fn build_engine(&self) {
        self.set_busy(true, "Building TagSpeak engine… this can take a minute");

        // Locate repo root: ...\tagspeak_rs\target\release\setup.exe -> pop 3
        let repo = match std::env::current_exe().ok().and_then(|mut p| {
            for _ in 0..3 {
                let _ = p.pop();
            }
            Some(p)
        }) {
            Some(p) => p,
            None => {
                self.set_busy(false, "Couldn’t resolve repo root");
                return;
            }
        };

        // sanity check
        let mut cargo_toml = repo.clone();
        cargo_toml.push("Cargo.toml");
        if !cargo_toml.exists() {
            self.set_busy(false, "Cargo.toml not found at repo root");
            return;
        }

        // find `cargo` (PATH or common location)
        let cargo = find_cargo();

        let status = std::process::Command::new(cargo)
            .current_dir(&repo)
            .args(["build", "--release", "-p", "tagspeak_rs"])
            .status();

        match status {
            Ok(s) if s.success() => {
                let mut exe = repo.clone();
                exe.push(r"target\release\tagspeak_rs.exe");
                if exe.exists() {
                    self.tb_engine.set_text(&exe.display().to_string());
                    self.update_enabled();
                    self.set_busy(false, "Build complete ✔");
                } else {
                    self.set_busy(false, "Build succeeded, but tagspeak_rs.exe not found");
                }
            }
            Ok(s) => {
                // [myth] goal: surface the numeric exit code
                let code = s
                    .code()
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "unknown".into());
                self.set_busy(false, &format!("Build failed (exit code {code})"));
            }
            Err(e) => {
                self.set_busy(false, &format!("Couldn’t launch cargo: {e}"));
            }
        }
    }

    fn set_busy(&self, busy: bool, msg: &str) {
        // quick UX: disable buttons during work + show status message
        self.btn_browse.set_enabled(!busy);
        self.btn_install.set_enabled(!busy);
        self.btn_uninstall.set_enabled(!busy);
        self.btn_run.set_enabled(!busy);
        self.btn_build.set_enabled(!busy);
        self.lbl_status.set_text(msg);
    }

    fn exit(&self) {
        nwg::stop_thread_dispatch();
    }
}

#[cfg(target_os = "windows")]
fn find_cargo() -> std::path::PathBuf {
    use std::path::PathBuf;
    // Try PATH first
    if let Ok(path) = which::which("cargo") {
        return path;
    }
    // Fallback to typical Windows install
    let mut p = dirs::home_dir().unwrap_or_else(|| PathBuf::from("C:\\"));
    p.push(r".cargo\bin\cargo.exe");
    p
}

#[cfg(target_os = "windows")]
fn main() {
    nwg::init().expect("NWG init failed");
    nwg::Font::set_global_family("Segoe UI").ok();

    let app = App::build_ui(Default::default()).expect("UI build failed");

    // 👑 set window icon from embedded .ico
    if let Ok(icon) = nwg::Icon::from_bin(include_bytes!("../../misc/Tagspeak.ico")) {
        app.window.set_icon(Some(&icon));
    }

    app.init_defaults();
    app.post_init_cleanup();
    nwg::dispatch_thread_events();
}

#[cfg(target_os = "windows")]
fn has_cargo() -> bool {
    if which::which("cargo").is_ok() { return true; }
    let mut p = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("C:\\"));
    p.push(r".cargo\bin\cargo.exe");
    p.exists()
}

#[cfg(not(target_os = "windows"))]
fn main() {}

/* ---------- registry helpers ---------- */
#[cfg(target_os = "windows")]
fn do_install(engine_exe: PathBuf) -> Result<()> {
    if !engine_exe.exists() {
        anyhow::bail!("Engine exe not found: {}", engine_exe.display());
    }
    let engine = engine_exe.display().to_string();
    // Write a user-local copy of Tagspeak.ico and use it for the filetype icon
    let default_icon = match write_user_icon() {
        Ok(path) => path.display().to_string(),
        Err(_) => format!("{},0", engine), // fallback to exe icon
    };

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let classes = hkcu.create_subkey("Software\\Classes")?.0;

    // Map extension -> ProgID
    let ext_key = classes.create_subkey(".tgsk")?.0;
    ext_key.set_value("", &PROGID)?;
    // Helpful metadata so Windows treats it like text
    let _ = ext_key.set_value("PerceivedType", &"text");
    let _ = ext_key.set_value("Content Type", &"text/plain");
    // Make TagSpeak appear in "Open with" list
    let open_with = classes.create_subkey(".tgsk\\OpenWithProgids")?.0;
    let _: Result<()> = open_with.set_value(PROGID, &"").map_err(|e| e.into());

    let prog = classes.create_subkey(PROGID)?.0;
    prog.set_value("", &DISPLAY)?;

    let icon_key = prog.create_subkey("DefaultIcon")?.0;
    icon_key.set_value("", &default_icon)?;

    let cmd_key = prog.create_subkey("shell\\open\\command")?.0;
    let command = format!("\"{}\" \"%1\"", engine);
    cmd_key.set_value("", &command)?;

    // Also register the application entry so "Open with" can find the exe
    if let Some(exe_name) = std::path::Path::new(&engine).file_name().and_then(|s| s.to_str()) {
        let app_key_path = format!("Applications\\{}\\shell\\open\\command", exe_name);
        let app_cmd = classes.create_subkey(&app_key_path)?.0;
        app_cmd.set_value("", &command)?;
        // Declare supported type for the application
        let supp = format!("Applications\\{}\\SupportedTypes", exe_name);
        let supp_key = classes.create_subkey(&supp)?.0;
        let _: Result<()> = supp_key.set_value(".tgsk", &"").map_err(|e| e.into());
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn write_user_icon() -> Result<PathBuf> {
    use std::fs;
    // Prefer Roaming AppData, fall back to Local if needed
    let base = dirs::data_dir()
        .or_else(|| dirs::data_local_dir())
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_else(|| PathBuf::from("C:/")).join("AppData/Local"));
    let dir = base.join("TagSpeak");
    fs::create_dir_all(&dir)?;
    let path = dir.join("tagspeak.ico");
    // Write embedded icon bytes
    fs::write(&path, include_bytes!("../../misc/Tagspeak.ico"))?;
    Ok(path)
}

#[cfg(target_os = "windows")]
fn do_uninstall() -> Result<()> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let classes = hkcu.open_subkey_with_flags("Software\\Classes", KEY_ALL_ACCESS)?;
    // Remove the extension mapping and the TagSpeak ProgID subtree if present
    let _ = classes.delete_subkey_all(".tgsk");
    let _ = classes.delete_subkey_all(PROGID);

    // Also clear Explorer per-user association cache for .tgsk
    if let Ok(file_exts) = hkcu.open_subkey_with_flags(
        "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\FileExts",
        KEY_ALL_ACCESS,
    ) {
        let _ = file_exts.delete_subkey_all(".tgsk");
    }
    Ok(())
}

/// Notify Windows that file associations changed so icons refresh.
///
/// # Errors
/// Currently returns `Ok(())` because [`SHChangeNotify`] has no error reporting.
#[cfg(target_os = "windows")]
fn refresh_icons() -> Result<()> {
    // [myth] goal: refresh icons without tanking Explorer
    // [myth] tradeoff: if the call fails, we don't get feedback
    // SAFETY: pointers are null and flags are documented constants
    unsafe {
        SHChangeNotify(
            SHCNE_ASSOCCHANGED as i32,
            SHCNF_IDLIST,
            std::ptr::null::<std::ffi::c_void>(),
            std::ptr::null::<std::ffi::c_void>(),
        );
    }
    Ok(())
}
