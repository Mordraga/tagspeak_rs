use std::io::{self, Read, Write};

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
mod wizard {
    use anyhow::{Context, Result, anyhow};
    use std::collections::HashSet;
    use std::fs;
    use std::io::{self, Write};
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use winreg::RegKey;
    use winreg::enums::*;

    use windows_sys::Win32::UI::Shell::{SHCNE_ASSOCCHANGED, SHCNF_IDLIST, SHChangeNotify};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        HWND_BROADCAST, SMTO_ABORTIFHUNG, SendMessageTimeoutW, WM_SETTINGCHANGE,
    };

    const ENGINE_HINT_FILE: &str = "engine_hint.txt";

    pub fn run() -> Result<()> {
        println!("========================================");
        println!("        TagSpeak Setup Wizard");
        println!("========================================\n");

        loop {
            println!("What would you like to do today?");
            println!("  1) Install or update TagSpeak");
            println!("  2) Uninstall TagSpeak");
            println!("  3) Build/refresh the engine binary");
            println!("  4) Exit");

            let choice = prompt("\nChoose an option (1-4): ")?;
            match choice.as_str() {
                "1" => install_flow()?,
                "2" => uninstall_flow()?,
                "3" => build_engine_flow()?,
                "4" => {
                    println!("\nThanks for visiting the TagSpeak wizard. See you next time!");
                    return Ok(());
                }
                _ => println!("\nPlease enter 1, 2, 3, or 4.\n"),
            }
        }
    }

    fn install_flow() -> Result<()> {
        println!("\n--- Step 1: Find the TagSpeak engine (tagspeak_rs.exe) ---\n");
        let mut engine = pick_engine_path()?;
        println!("Using engine: {}\n", engine.display());

        println!("--- Step 2: Choose an installation folder ---\n");
        if yes_no(
            "Copy the engine into a dedicated TagSpeak install folder? [Y/n]: ",
            true,
        )? {
            engine = copy_engine_to_install_dir(&engine)?;
        } else {
            println!("Keeping the engine where it is ({}).", engine.display());
        }

        println!("\n--- Step 3: Installation options ---\n");
        let register_assoc = yes_no("Associate .tgsk files with TagSpeak? [Y/n]: ", true)?;
        let install_cli = yes_no("Install the `tagspeak` command-line helper? [Y/n]: ", true)?;

        println!("\n--- Step 4: Confirm ---\n");
        println!("  Engine path       : {}", engine.display());
        if let Some(parent) = engine.parent() {
            println!("  Install folder    : {}", parent.display());
        }
        println!("  File association  : {}", pretty_bool(register_assoc));
        println!("  CLI helper        : {}", pretty_bool(install_cli));

        if !yes_no("\nProceed with these changes? [Y/n]: ", true)? {
            println!("\nNo changes were made.\n");
            return Ok(());
        }

        let engine = fs::canonicalize(&engine)
            .with_context(|| format!("Failed to resolve {}", engine.display()))?;

        if register_assoc {
            if let Err(err) = register_file_assoc(&engine) {
                return Err(decorate_permission_error(
                    err,
                    "Registering .tgsk file associations",
                ));
            }
        }
        if install_cli {
            if let Err(err) = install_cli_alias(&engine) {
                return Err(decorate_permission_error(
                    err,
                    "Installing the TagSpeak CLI helper",
                ));
            }
        }

        if let Err(err) = write_engine_hint(&engine) {
            return Err(decorate_permission_error(err, "Saving the engine location"));
        }
        println!("\nTagSpeak is ready to go. Enjoy!\n");
        Ok(())
    }

    fn uninstall_flow() -> Result<()> {
        println!("\nThis will remove the .tgsk association and CLI helper.");
        if !yes_no("Continue? [y/N]: ", false)? {
            println!("\nNo changes were made.\n");
            return Ok(());
        }

        unregister_file_assoc()?;
        remove_cli_alias()?;
        println!("\nTagSpeak was removed from file associations and PATH.\n");
        Ok(())
    }

    fn build_engine_flow() -> Result<()> {
        if !has_cargo() {
            println!(
                "\nCargo (Rust) is not available. Install it from https://rustup.rs, \
then run this option again.\n"
            );
            return Ok(());
        }

        println!("\nBuilding TagSpeak engine (cargo build --release)...\n");
        let status = Command::new("cargo")
            .args(["build", "--release", "-p", "tagspeak_rs"])
            .status()
            .context("Failed to run cargo build")?;
        if status.success() {
            println!("Build finished successfully.\n");
            if let Some(built) = discover_built_engine() {
                println!("Built engine: {}", built.display());
                if yes_no(
                    "Copy this build into your TagSpeak install folder now? [Y/n]: ",
                    true,
                )? {
                    let _ = copy_engine_to_install_dir(&built)?;
                }
            }
        } else {
            println!("Build failed (status {}).\n", status);
        }
        Ok(())
    }

    fn pick_engine_path() -> Result<PathBuf> {
        loop {
            let mut options = discover_engine_candidates();
            options.sort();

            if options.is_empty() {
                println!("Couldn't automatically find tagspeak_rs.exe.");
            } else {
                println!("Found these candidates:");
                for (idx, path) in options.iter().enumerate() {
                    println!("  {}) {}", idx + 1, path.display());
                }
            }
            println!("  C) Choose a custom path");
            println!("  B) Build the engine now");

            let choice = prompt("\nSelect an option: ")?;
            if let Ok(num) = choice.parse::<usize>() {
                if (1..=options.len()).contains(&num) {
                    let candidate = &options[num - 1];
                    if candidate.exists() {
                        if let Some(resolved) = resolve_engine_candidate(candidate) {
                            return Ok(resolved);
                        }
                        println!(
                            "That selection is a folder without a tagspeak_rs.exe. Let's try again.\n"
                        );
                        continue;
                    }
                    println!("That path no longer exists. Let's try again.\n");
                    continue;
                }
            }

            match choice.to_uppercase().as_str() {
                "C" => {
                    let custom = prompt("Enter the full path to tagspeak_rs.exe: ")?;
                    let trimmed = trim_quotes(&custom);
                    if trimmed.is_empty() {
                        println!("Please provide a path.\n");
                        continue;
                    }
                    let path = PathBuf::from(&trimmed);
                    if !path.exists() {
                        println!("That path doesn't exist. Try again.\n");
                        continue;
                    }
                    if let Some(resolved) = resolve_engine_candidate(&path) {
                        return Ok(resolved);
                    }
                    if path.is_dir() {
                        println!(
                            "That folder doesn't contain tagspeak_rs.exe. Build the engine first or point directly at the executable.\n"
                        );
                    } else {
                        println!(
                            "That file isn't the TagSpeak engine (tagspeak_rs.exe). Try again.\n"
                        );
                    }
                }
                "B" => {
                    build_engine_flow()?;
                }
                _ => println!("Please choose one of the listed options.\n"),
            }
        }
    }

    fn prompt(message: &str) -> Result<String> {
        print!("{message}");
        io::stdout().flush().ok();
        let mut buf = String::new();
        io::stdin()
            .read_line(&mut buf)
            .map_err(|e| anyhow!("Failed to read input: {e}"))?;
        Ok(buf.trim().to_string())
    }

    fn yes_no(message: &str, default_yes: bool) -> Result<bool> {
        loop {
            let answer = prompt(message)?;
            if answer.is_empty() {
                return Ok(default_yes);
            }
            match answer.to_lowercase().as_str() {
                "y" | "yes" => return Ok(true),
                "n" | "no" => return Ok(false),
                _ => println!("Please answer yes or no (y/n)."),
            }
        }
    }

    fn discover_engine_candidates() -> Vec<PathBuf> {
        let mut seen = HashSet::new();
        let mut out = Vec::new();

        let mut push_candidate = |path: PathBuf| {
            if let Some(valid) = resolve_engine_candidate(&path) {
                let key = normalize_path(&valid);
                if seen.insert(key) {
                    out.push(valid);
                }
            }
        };

        if let Some(saved) = read_engine_hint() {
            push_candidate(saved);
        }
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                push_candidate(dir.join("tagspeak_rs.exe"));
                if let Some(parent) = dir.parent() {
                    push_candidate(parent.join("tagspeak_rs.exe"));
                }
            }
        }
        if let Ok(found) = which::which("tagspeak_rs") {
            push_candidate(found);
        }
        if let Ok(cwd) = std::env::current_dir() {
            push_candidate(cwd.join("target\\release\\tagspeak_rs.exe"));
            push_candidate(cwd.join("target\\debug\\tagspeak_rs.exe"));
        }

        out
    }

    fn resolve_engine_candidate<P: AsRef<Path>>(candidate: P) -> Option<PathBuf> {
        let path = candidate.as_ref();
        if path.is_file() {
            if path
                .file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.eq_ignore_ascii_case("tagspeak_rs.exe"))
                .unwrap_or(false)
            {
                return fs::canonicalize(path).ok();
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

        if let Ok(file_exts) = hkcu.open_subkey_with_flags(
            "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\FileExts",
            KEY_ALL_ACCESS,
        ) {
            let _ = file_exts.delete_subkey_all(".tgsk");
        }

        refresh_icons();
        Ok(())
    }

    fn install_cli_alias(engine: &Path) -> Result<()> {
        let dir = tagspeak_data_dir();
        fs::create_dir_all(&dir)?;
        let alias = dir.join("tagspeak.exe");
        if alias.exists() {
            let _ = fs::remove_file(&alias);
        }
        fs::copy(engine, &alias)?;
        ensure_path_contains(&dir)?;
        Ok(())
    }

    fn remove_cli_alias() -> Result<()> {
        let dir = tagspeak_data_dir();
        let alias = dir.join("tagspeak.exe");
        if alias.exists() {
            let _ = fs::remove_file(&alias);
        }
        remove_from_path(&dir)?;
        Ok(())
    }

    fn tagspeak_data_dir() -> PathBuf {
        dirs::data_dir()
            .or_else(|| dirs::data_local_dir())
            .unwrap_or_else(|| {
                dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("C:\\"))
                    .join("AppData\\Local")
            })
            .join("TagSpeak")
    }

    fn write_engine_hint(engine: &Path) -> Result<()> {
        let dir = tagspeak_data_dir();
        fs::create_dir_all(&dir)?;
        let hint = dir.join(ENGINE_HINT_FILE);
        fs::write(hint, engine.display().to_string())?;
        Ok(())
    }

    fn read_engine_hint() -> Option<PathBuf> {
        let hint = tagspeak_data_dir().join(ENGINE_HINT_FILE);
        fs::read_to_string(hint).ok().map(PathBuf::from)
    }

    fn write_user_icon() -> Result<String> {
        let dir = tagspeak_data_dir();
        fs::create_dir_all(&dir)?;
        let icon = dir.join("tagspeak.ico");
        fs::write(&icon, include_bytes!("../../misc/Tagspeak.ico"))?;
        Ok(icon.display().to_string())
    }

    fn ensure_path_contains(dir: &Path) -> Result<()> {
        let canonical = normalize_path(dir);
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let env = hkcu.open_subkey_with_flags("Environment", KEY_READ | KEY_WRITE)?;
        let mut path: String = env.get_value("Path").unwrap_or_default();
        let mut entries: Vec<String> = path
            .split(';')
            .filter_map(|s| {
                let trimmed = s.trim();
                (!trimmed.is_empty()).then(|| trimmed.to_string())
            })
            .collect();

        if entries.iter().any(|p| normalize_str(p) == canonical) {
            return Ok(());
        }

        entries.push(dir.display().to_string());
        path = entries.join(";");
        env.set_value("Path", &path)?;
        broadcast_env_change();
        Ok(())
    }

    fn remove_from_path(dir: &Path) -> Result<()> {
        let canonical = normalize_path(dir);
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let env = hkcu.open_subkey_with_flags("Environment", KEY_READ | KEY_WRITE)?;
        let path: String = env.get_value("Path").unwrap_or_default();
        let mut entries: Vec<String> = path
            .split(';')
            .filter_map(|s| {
                let trimmed = s.trim();
                (!trimmed.is_empty()).then(|| trimmed.to_string())
            })
            .collect();
        let original_len = entries.len();
        entries.retain(|p| normalize_str(p) != canonical);
        if entries.len() != original_len {
            env.set_value("Path", &entries.join(";"))?;
            broadcast_env_change();
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
