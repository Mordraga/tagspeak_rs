#![windows_subsystem = "windows"]
use anyhow::Result;
use native_windows_gui as nwg;
use native_windows_derive as nwd;
use nwg::NativeUi;
use std::{path::PathBuf, process::Command};
use winreg::{enums::*, RegKey};

const PROGID: &str = "TagSpeakFile";
const DISPLAY: &str = "TagSpeak Script";

#[derive(Default, nwd::NwgUi)]
pub struct App {
    #[nwg_control(size: (560, 260), title: "TagSpeak Setup", flags: "WINDOW|VISIBLE")]
    #[nwg_events(OnWindowClose: [App::exit])]
    window: nwg::Window,

    // this is the layout control the macro items bind to
    #[nwg_layout(parent: window)]
    layout: nwg::GridLayout,

    #[nwg_control(text: "Hey! This is TagSpeak.\nPick your engine exe and install .tgsk")]
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

impl App {
    fn init_defaults(&self) {
        // try to auto-fill engine path (…\tagspeak_rs\target\release\tagspeak_rs.exe)
        if let Ok(mut here) = std::env::current_exe() {
            for _ in 0..2 { here = here.parent().unwrap().to_path_buf(); } // up from target\release
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
        self.lbl_status.set_text("Engine not found. Click “Build engine”.");
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
            if let Ok(os) = dialog.get_selected_item() {
                let p = std::path::PathBuf::from(os);
                self.tb_engine.set_text(&p.display().to_string());
                self.update_enabled();
            }
        }
    }

    fn install(&self) {
        let exe = std::path::PathBuf::from(self.tb_engine.text());
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
        let repo = match std::env::current_exe()
            .ok()
            .and_then(|mut p| { for _ in 0..3 { let _ = p.pop(); } Some(p) })
        {
            Some(p) => p,
            None => { self.set_busy(false, "Couldn’t resolve repo root"); return; }
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
                self.set_busy(false, &format!("Build failed (exit code {s})"));
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


    fn exit(&self) { nwg::stop_thread_dispatch(); }
}

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


fn main() {
    nwg::init().expect("NWG init failed");
    nwg::Font::set_global_family("Segoe UI").ok();

    let app = App::build_ui(Default::default()).expect("UI build failed");

    // 👑 set window icon from embedded .ico
    if let Ok(icon) = nwg::Icon::from_bin(include_bytes!("../../misc/Tagspeak.ico")) {
        app.window.set_icon(Some(&icon));
    }

    app.init_defaults();
    nwg::dispatch_thread_events();
}


/* ---------- registry helpers ---------- */
fn do_install(engine_exe: PathBuf) -> Result<()> {
    if !engine_exe.exists() { anyhow::bail!("Engine exe not found: {}", engine_exe.display()); }
    let engine = engine_exe.display().to_string();
    let default_icon = format!("{},0", engine);

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let classes = hkcu.create_subkey("Software\\Classes")?.0;

    let ext_key = classes.create_subkey(".tgsk")?.0;
    ext_key.set_value("", &PROGID)?;

    let prog = classes.create_subkey(PROGID)?.0;
    prog.set_value("", &DISPLAY)?;

    let icon_key = prog.create_subkey("DefaultIcon")?.0;
    icon_key.set_value("", &default_icon)?;

    let cmd_key = prog.create_subkey("shell\\open\\command")?.0;
    let command = format!("\"{}\" \"%1\"", engine);
    cmd_key.set_value("", &command)?;
    Ok(())
}

fn do_uninstall() -> Result<()> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let classes = hkcu.open_subkey_with_flags("Software\\Classes", KEY_ALL_ACCESS)?;
    if let Ok(ext_key) = classes.open_subkey_with_flags(".tgsk", KEY_SET_VALUE) {
        let _ = ext_key.delete_value("");
    }
    Ok(())
}

fn refresh_icons() -> Result<()> {
    let _ = Command::new("ie4uinit.exe").arg("-ClearIconCache").status();
    let _ = Command::new("taskkill").args(["/IM","explorer.exe","/F"]).status();
    let _ = Command::new("explorer.exe").status();
    Ok(())
}
