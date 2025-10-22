#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

#[cfg(target_os = "windows")]
fn main() {
    let result = wizard::run();
    if let Err(err) = &result {
        eprintln!("\nSetup finished with an error:\n{err:#}");
    }
    println!("\nPress Enter to close the TagSpeak wizard...");
    let _ = io::stdin().read(&mut [0u8]).ok();
}

#[cfg(not(target_os = "windows"))]
fn main() {
    println!("TagSpeak setup is only required on Windows. You're already good to go!");
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
                    self.set_busy(
                        false,
                        "Build finished but engine not found. Select it manually or try again.",
                    );
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
                    .args(["url.dll,FileProtocolHandler", "https://rustup.rs"])
                    .spawn();
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
            return None;
        }

        if path.is_dir() {
            let search_targets = [
                Path::new("tagspeak_rs.exe"),
                Path::new("target\\release\\tagspeak_rs.exe"),
                Path::new("target\\debug\\tagspeak_rs.exe"),
            ];
            for rel in search_targets {
                let candidate = path.join(rel);
                if candidate.is_file() {
                    return fs::canonicalize(candidate).ok();
                }
            }
        }

        None
    }

    fn has_cargo() -> bool {
        if which::which("cargo").is_ok() {
            return true;
        }
        let mut path = dirs::home_dir().unwrap_or_else(|| PathBuf::from("C:\\"));
        path.push(r".cargo\bin\cargo.exe");
        path.exists()
    }

    fn pretty_bool(value: bool) -> &'static str {
        if value { "Yes" } else { "No" }
    }

    fn trim_quotes(input: &str) -> String {
        input.trim().trim_matches('"').to_string()
    }

    fn copy_engine_to_install_dir(engine: &Path) -> Result<PathBuf> {
        let default_dir = default_install_dir();
        println!("Where should TagSpeak keep its engine binary?");
        println!(
            "Press Enter to accept the default: {}",
            default_dir.display()
        );
        let dest_input = prompt("Install folder: ")?;
        let dest_dir = if dest_input.is_empty() {
            default_dir
        } else {
            PathBuf::from(trim_quotes(&dest_input))
        };

        fs::create_dir_all(&dest_dir)
            .with_context(|| format!("Failed to create install directory {}", dest_dir.display()))
            .map_err(|err| decorate_permission_error(err, "Creating the install folder"))?;
        let target = dest_dir.join("tagspeak_rs.exe");
        fs::copy(engine, &target)
            .with_context(|| format!("Failed to copy engine to {}", target.display()))
            .map_err(|err| decorate_permission_error(err, "Copying the engine binary"))?;
        let canonical = fs::canonicalize(&target)
            .with_context(|| format!("Failed to resolve {}", target.display()))?;
        println!("Copied engine to {}", canonical.display());
        Ok(canonical)
    }

    fn default_install_dir() -> PathBuf {
        tagspeak_data_dir().join("engine")
    }

    fn discover_built_engine() -> Option<PathBuf> {
        let mut path = std::env::current_dir().ok()?;
        path.push("target\\release\\tagspeak_rs.exe");
        path.exists().then_some(path)
    }

    fn register_file_assoc(engine: &Path) -> Result<()> {
        if !engine.exists() {
            return Err(anyhow!("Engine executable not found: {}", engine.display()));
        }
        let engine_str = engine.display().to_string();
        let icon_path = write_user_icon().unwrap_or_else(|_| engine_str.clone());

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let classes = hkcu.create_subkey("Software\\Classes")?.0;

        let ext = classes.create_subkey(".tgsk")?.0;
        ext.set_value("", &"TagSpeakFile")?;
        let _ = ext.set_value("PerceivedType", &"text");
        let _ = ext.set_value("Content Type", &"text/plain");
        classes
            .create_subkey(".tgsk\\OpenWithProgids")?
            .0
            .set_value("TagSpeakFile", &"")?;

        let prog = classes.create_subkey("TagSpeakFile")?.0;
        prog.set_value("", &"TagSpeak Script")?;
        prog.create_subkey("DefaultIcon")?
            .0
            .set_value("", &icon_path)?;

        let command = format!("\"{engine_str}\" \"%1\"");
        prog.create_subkey("shell\\open\\command")?
            .0
            .set_value("", &command)?;

        if let Some(exe_name) = engine.file_name().and_then(|s| s.to_str()) {
            let app_key_path = format!("Applications\\{}\\shell\\open\\command", exe_name);
            classes
                .create_subkey(&app_key_path)?
                .0
                .set_value("", &command)?;
            let supp_key = format!("Applications\\{}\\SupportedTypes", exe_name);
            classes
                .create_subkey(&supp_key)?
                .0
                .set_value(".tgsk", &"")?;
        }

        refresh_icons();
        Ok(())
    }

    fn unregister_file_assoc() -> Result<()> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let classes = hkcu.open_subkey_with_flags("Software\\Classes", KEY_ALL_ACCESS)?;
        let _ = classes.delete_subkey_all(".tgsk");
        let _ = classes.delete_subkey_all("TagSpeakFile");

    // Also clear Explorer per-user association cache for .tgsk
    if let Ok(file_exts) = hkcu.open_subkey_with_flags(
        "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\FileExts",
        KEY_ALL_ACCESS,
    ) {
        let _ = file_exts.delete_subkey_all(".tgsk");
    }
    Ok(())
}

    fn normalize_path(path: &Path) -> String {
        normalize_str(&path.display().to_string())
    }

    fn normalize_str(entry: &str) -> String {
        entry
            .trim()
            .trim_matches('"')
            .replace('/', "\\")
            .trim_end_matches('\\')
            .to_lowercase()
    }

    fn decorate_permission_error(err: anyhow::Error, action: &str) -> anyhow::Error {
        if is_permission_denied(&err) {
            err.context(format!(
                "{action} requires elevated permissions or a writable folder. \
Try rerunning the wizard as Administrator or choose a directory within your user profile."
            ))
        } else {
            err
        }
    }

    fn is_permission_denied(err: &anyhow::Error) -> bool {
        err.chain().any(|cause| {
            cause
                .downcast_ref::<io::Error>()
                .map(|io_err| io_err.kind() == io::ErrorKind::PermissionDenied)
                .unwrap_or(false)
        })
    }

    fn refresh_icons() {
        unsafe {
            SHChangeNotify(
                SHCNE_ASSOCCHANGED as i32,
                SHCNF_IDLIST,
                std::ptr::null(),
                std::ptr::null(),
            );
        }
    }

    fn broadcast_env_change() {
        let mut payload = "Environment\0".encode_utf16().collect::<Vec<u16>>();
        unsafe {
            SendMessageTimeoutW(
                HWND_BROADCAST,
                WM_SETTINGCHANGE,
                0,
                payload.as_mut_ptr() as isize,
                SMTO_ABORTIFHUNG,
                5000,
                std::ptr::null_mut(),
            );
        }
    }
}
