use anyhow::{Result, anyhow};
use clap::{ArgAction, Parser};
use std::path::{Path, PathBuf};

#[cfg(target_os = "windows")]
use std::fs;
#[cfg(target_os = "windows")]
use std::io::{self, Write};
#[cfg(target_os = "windows")]
use std::os::windows::ffi::OsStrExt;
#[cfg(target_os = "windows")]
use std::process::Command;

#[derive(Parser, Debug)]
#[command(
    name = "tagspeak_setup",
    about = "Terminal setup for TagSpeak engine and file associations"
)]
struct Opts {
    /// Show current handler info
    #[arg(long, action = ArgAction::SetTrue)]
    check: bool,

    /// Associate .tgsk with the given engine (path to tagspeak_rs)

    #[arg(long)]
    associate: Option<PathBuf>,

    /// Print steps for association but do nothing

    #[arg(long, requires = "associate", action = ArgAction::SetTrue)]
    associate_dry: bool,

    /// Remove association

    #[arg(long, action = ArgAction::SetTrue)]
    uninstall: bool,

    /// Print steps for uninstall but do nothing

    #[arg(long, requires = "uninstall", action = ArgAction::SetTrue)]
    uninstall_dry: bool,
}

fn main() -> Result<()> {
    let opts = Opts::parse();

    #[cfg(target_os = "windows")]
    {
        if opts.check {
            return cmd_check();
        }
        if let Some(engine) = opts.associate.as_ref() {
            return cmd_associate(engine, opts.associate_dry);
        }
        if opts.uninstall {
            return cmd_uninstall(opts.uninstall_dry);
        }
        return cmd_wizard();
    }

    #[cfg(not(target_os = "windows"))]
    {
        if opts.check {
            return cmd_check();
        }
        if let Some(engine) = opts.associate.as_ref() {
            return cmd_associate(engine, opts.associate_dry);
        }
        if opts.uninstall {
            return cmd_uninstall(opts.uninstall_dry);
        }
        eprintln!(
            "Usage:\n  tagspeak_setup --check\n  tagspeak_setup --associate <ENGINE> [--associate-dry]\n  tagspeak_setup --uninstall [--uninstall-dry]"
        );
        Ok(())
    }
}

#[cfg(target_os = "windows")]
fn cmd_wizard() -> Result<()> {
    loop {
        println!();
        println!("=== TagSpeak Setup Wizard ===");
        println!("1) Build and install the TagSpeak engine");
        println!("2) Uninstall TagSpeak and remove the engine");
        println!("3) Show current file association");
        println!("4) Exit");
        let choice = prompt_line("Select an option [1-4]: ")?;
        match choice.as_str() {
            "1" => handle_action(run_install_flow)?,
            "2" => handle_action(run_uninstall_flow)?,
            "3" => handle_action(cmd_check)?,
            "4" => {
                println!("Goodbye.");
                return Ok(());
            }
            _ => println!("Please enter 1, 2, 3, or 4."),
        }
    }
}

#[cfg(target_os = "windows")]
fn run_install_flow() -> Result<()> {
    println!();
    println!("--- Install / Update TagSpeak ---");

    let repo_root = select_repo_root()?;
    let default_dir = default_install_dir(&repo_root)?;
    let default_display = default_dir.display().to_string();
    let target_input = prompt_with_default("Install directory", &default_display)?;
    let target_dir = resolve_install_dir(&target_input, &repo_root)?;

    fs::create_dir_all(&target_dir)?;
    let target_dir = match fs::canonicalize(&target_dir) {
        Ok(path) => path,
        Err(_) => target_dir,
    };
    let engine_path = target_dir.join("tagspeak_rs.exe");
    if engine_path.exists()
        && !prompt_yes_no(
            &format!("{} already exists. Overwrite?", engine_path.display()),
            true,
        )?
    {
        println!("Install cancelled.");
        return Ok(());
    }

    println!("Running `cargo build --release`...");
    let status = Command::new("cargo")
        .args(["build", "--release"])
        .current_dir(&repo_root)
        .status()
        .map_err(|err| anyhow!("Failed to spawn cargo build: {err}"))?;
    if !status.success() {
        return Err(anyhow!("cargo build failed with status {}", status));
    }

    let built_engine = repo_root.join("target/release/tagspeak_rs.exe");
    if !built_engine.exists() {
        return Err(anyhow!(
            "Built engine not found at {}",
            built_engine.display()
        ));
    }

    fs::copy(&built_engine, &engine_path)
        .map_err(|err| anyhow!("Failed to copy engine to {}: {err}", engine_path.display()))?;
    println!("Installed engine to {}", engine_path.display());

    let path_changed = ensure_path_entry(&target_dir)?;
    if path_changed {
        broadcast_environment_change();
        println!("Added {} to your user PATH.", target_dir.display());
    } else {
        println!("{} is already on your user PATH.", target_dir.display());
    }

    store_install_dir(&target_dir)?;
    cmd_associate(&engine_path, false)?;

    println!("Install complete. Restart open terminals to pick up the updated PATH.");
    Ok(())
}

#[cfg(target_os = "windows")]
fn run_uninstall_flow() -> Result<()> {
    println!();
    println!("--- Uninstall TagSpeak ---");

    let stored_dir = read_install_dir()?;
    let repo_root = detect_repo_root().ok();
    let suggested = stored_dir.clone().or_else(|| {
        repo_root
            .as_ref()
            .and_then(|root| default_install_dir(root).ok())
    });
    let current_dir = std::env::current_dir()?;

    let mut install_dir = if let Some(default_path) = suggested {
        let default_display = default_path.display().to_string();
        let chosen = prompt_with_default("Install directory to remove", &default_display)?;
        resolve_install_dir(&chosen, &current_dir)?
    } else {
        let entered = prompt_line("Install directory to remove: ")?;
        if entered.trim().is_empty() {
            return Err(anyhow!("Install directory cannot be empty."));
        }
        resolve_install_dir(&entered, &current_dir)?
    };
    install_dir = match fs::canonicalize(&install_dir) {
        Ok(path) => path,
        Err(_) => install_dir,
    };

    if !prompt_yes_no(
        &format!(
            "Remove engine at {} and clean associations?",
            install_dir.display()
        ),
        true,
    )? {
        println!("Uninstall cancelled.");
        return Ok(());
    }

    if install_dir.exists() {
        fs::remove_dir_all(&install_dir).map_err(|err| {
            anyhow!(
                "Failed to remove install directory {}: {err}",
                install_dir.display()
            )
        })?;
        println!("Removed {}", install_dir.display());
    } else {
        println!(
            "Install directory {} does not exist, skipping removal.",
            install_dir.display()
        );
    }

    let path_changed = remove_path_entry(&install_dir)?;
    if path_changed {
        broadcast_environment_change();
        println!("Removed {} from your user PATH.", install_dir.display());
    }

    cmd_uninstall(false)?;
    clear_install_dir_record()?;
    println!("Uninstall complete.");
    Ok(())
}

#[cfg(target_os = "windows")]
fn detect_repo_root() -> Result<PathBuf> {
    let cwd = std::env::current_dir()?;
    if cwd.join("Cargo.toml").exists() {
        Ok(cwd)
    } else {
        Err(anyhow!(
            "Could not find Cargo.toml in the current directory ({}). Run the setup from the TagSpeak repository root.",
            cwd.display()
        ))
    }
}

#[cfg(target_os = "windows")]
fn select_repo_root() -> Result<PathBuf> {
    if let Ok(root) = detect_repo_root() {
        return Ok(root);
    }
    println!("Could not locate Cargo.toml in the current directory.");
    println!("Enter the path to your TagSpeak repository (the folder with Cargo.toml).");
    loop {
        let input = prompt_line("Repository path: ")?;
        if input.trim().is_empty() {
            println!("Please provide a path.");
            continue;
        }
        let candidate = PathBuf::from(input.trim());
        let candidate = if candidate.is_absolute() {
            candidate
        } else {
            std::env::current_dir()?.join(candidate)
        };
        let candidate = match fs::canonicalize(&candidate) {
            Ok(path) => path,
            Err(_) => candidate,
        };
        if candidate.join("Cargo.toml").exists() {
            return Ok(candidate);
        }
        println!(
            "Cargo.toml not found at {}. Try again.",
            candidate.display()
        );
    }
}

#[cfg(target_os = "windows")]
fn default_install_dir(repo_root: &Path) -> Result<PathBuf> {
    if let Some(mut dir) = dirs::data_local_dir() {
        dir.push("TagSpeak");
        Ok(dir)
    } else {
        Ok(repo_root.join("tagspeak_engine"))
    }
}

#[cfg(target_os = "windows")]
fn resolve_install_dir(input: &str, base: &Path) -> Result<PathBuf> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("Install directory cannot be empty."));
    }
    let candidate = PathBuf::from(trimmed);
    let resolved = if candidate.is_absolute() {
        candidate
    } else {
        base.join(candidate)
    };
    Ok(resolved)
}

#[cfg(target_os = "windows")]
fn prompt_line(prompt: &str) -> Result<String> {
    print!("{prompt}");
    io::stdout().flush()?;
    let mut buf = String::new();
    io::stdin().read_line(&mut buf)?;
    Ok(buf.trim().to_string())
}

#[cfg(target_os = "windows")]
fn prompt_with_default(label: &str, default: &str) -> Result<String> {
    let prompt = format!("{label} [{default}]: ");
    let input = prompt_line(&prompt)?;
    if input.is_empty() {
        Ok(default.to_string())
    } else {
        Ok(input)
    }
}

#[cfg(target_os = "windows")]
fn prompt_yes_no(question: &str, default_yes: bool) -> Result<bool> {
    let suffix = if default_yes { "[Y/n]" } else { "[y/N]" };
    loop {
        let prompt = format!("{question} {suffix} ");
        let input = prompt_line(&prompt)?;
        if input.is_empty() {
            return Ok(default_yes);
        }
        match input.to_ascii_lowercase().as_str() {
            "y" | "yes" => return Ok(true),
            "n" | "no" => return Ok(false),
            _ => println!("Please answer yes or no."),
        }
    }
}

#[cfg(target_os = "windows")]
fn handle_action<F>(mut action: F) -> Result<()>
where
    F: FnMut() -> Result<()>,
{
    match action() {
        Ok(()) => {
            let _ = prompt_line("Press Enter to continue...");
        }
        Err(err) => {
            eprintln!("\n[error] {err}");
            let _ = prompt_line("Press Enter to return to the menu...");
        }
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn ensure_path_entry(dir: &Path) -> Result<bool> {
    use winreg::RegKey;
    use winreg::enums::HKEY_CURRENT_USER;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let env = hkcu.create_subkey("Environment")?.0;
    let current: String = env.get_value("Path").unwrap_or_default();
    let dir_str = dir.display().to_string();
    let norm_dir = normalize_windows_path(&dir_str);

    let mut entries: Vec<String> = current
        .split(';')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if entries
        .iter()
        .any(|entry| normalize_windows_path(entry) == norm_dir)
    {
        return Ok(false);
    }
    entries.push(dir_str.clone());
    let new_path = entries.join(";");
    env.set_value("Path", &new_path)?;
    Ok(true)
}

#[cfg(target_os = "windows")]
fn remove_path_entry(dir: &Path) -> Result<bool> {
    use winreg::RegKey;
    use winreg::enums::HKEY_CURRENT_USER;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let env = hkcu.create_subkey("Environment")?.0;
    let current: String = env.get_value("Path").unwrap_or_default();
    let dir_str = dir.display().to_string();
    let norm_dir = normalize_windows_path(&dir_str);

    let mut entries: Vec<String> = current
        .split(';')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let original_len = entries.len();
    entries.retain(|entry| normalize_windows_path(entry) != norm_dir);
    if entries.len() == original_len {
        return Ok(false);
    }
    let new_path = entries.join(";");
    env.set_value("Path", &new_path)?;
    Ok(true)
}

#[cfg(target_os = "windows")]
fn normalize_windows_path(value: &str) -> String {
    value
        .trim()
        .trim_end_matches(|c| c == '\\' || c == '/')
        .replace('/', "\\")
        .to_ascii_lowercase()
}

#[cfg(target_os = "windows")]
fn store_install_dir(dir: &Path) -> Result<()> {
    use winreg::RegKey;
    use winreg::enums::HKEY_CURRENT_USER;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = hkcu.create_subkey("Software\\TagSpeak")?.0;
    key.set_value("InstallDir", &dir.display().to_string())?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn read_install_dir() -> Result<Option<PathBuf>> {
    use winreg::RegKey;
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ};

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(key) = hkcu.open_subkey_with_flags("Software\\TagSpeak", KEY_READ) {
        let val: String = key.get_value("InstallDir")?;
        if val.trim().is_empty() {
            Ok(None)
        } else {
            Ok(Some(PathBuf::from(val)))
        }
    } else {
        Ok(None)
    }
}

#[cfg(target_os = "windows")]
fn clear_install_dir_record() -> Result<()> {
    use winreg::RegKey;
    use winreg::enums::{HKEY_CURRENT_USER, KEY_ALL_ACCESS};

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(software) = hkcu.open_subkey_with_flags("Software", KEY_ALL_ACCESS) {
        let _ = software.delete_subkey_all("TagSpeak");
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn broadcast_environment_change() {
    use std::ffi::OsStr;
    use std::iter;
    use windows_sys::Win32::Foundation::{LPARAM, WPARAM};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        HWND_BROADCAST, SMTO_ABORTIFHUNG, SendMessageTimeoutW, WM_SETTINGCHANGE,
    };

    let wide: Vec<u16> = OsStr::new("Environment")
        .encode_wide()
        .chain(iter::once(0))
        .collect();
    unsafe {
        SendMessageTimeoutW(
            HWND_BROADCAST,
            WM_SETTINGCHANGE,
            WPARAM::default(),
            wide.as_ptr() as LPARAM,
            SMTO_ABORTIFHUNG,
            5000,
            std::ptr::null_mut(),
        );
    }
}

#[cfg(target_os = "windows")]
fn cmd_check() -> Result<()> {
    use winreg::RegKey;
    use winreg::enums::HKEY_CURRENT_USER;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let classes = hkcu.open_subkey("Software\\Classes").ok();

    let prog = classes
        .as_ref()
        .and_then(|c| c.open_subkey("TagSpeakFile").ok());

    if let Some(prog) = prog {
        let cmd_key = prog.open_subkey("shell\\open\\command").ok();

        let cmd: Option<String> = cmd_key.as_ref().and_then(|k| k.get_value("").ok());

        println!(
            "Current handler: {}",
            cmd.unwrap_or_else(|| "<not set>".into())
        );
    } else {
        println!("No per-user association found.");
    }

    Ok(())
}

#[cfg(not(target_os = "windows"))]

fn cmd_check() -> Result<()> {
    println!("No setup required on this OS. Use your editor to open .tgsk files.");

    Ok(())
}

#[cfg(target_os = "windows")]
fn cmd_associate(engine: &Path, dry: bool) -> Result<()> {
    let engine = if engine.exists() {
        engine.to_path_buf()
    } else if let Ok(found) = which::which("tagspeak_rs") {
        found
    } else {
        return Err(anyhow!(
            "Engine not found: {} (and not on PATH)",
            engine.display()
        ));
    };

    let engine_str = engine.display().to_string();

    let command = format!("\"{engine_str}\" \"%1\"");

    if dry {
        println!("Would set per-user association for .tgsk -> {engine_str}");

        println!("  - HKCU/Software/Classes/.tgsk -> TagSpeakFile");

        println!("  - HKCU/Software/Classes/TagSpeakFile/shell/open/command -> {command}");

        return Ok(());
    }

    use winreg::RegKey;

    use winreg::enums::{HKEY_CURRENT_USER, KEY_ALL_ACCESS};

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
        .set_value("", &engine_str)?;

    prog.create_subkey("shell\\open\\command")?
        .0
        .set_value("", &command)?;

    // Explorer association cache cleanup (best-effort)

    if let Ok(file_exts) = hkcu.open_subkey_with_flags(
        "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\FileExts",
        KEY_ALL_ACCESS,
    ) {
        let _ = file_exts.delete_subkey_all(".tgsk");
    }

    refresh_icons();

    println!("Associated .tgsk with {engine_str}");

    Ok(())
}

#[cfg(not(target_os = "windows"))]

fn cmd_associate(_engine: &Path, _dry: bool) -> Result<()> {
    println!("Association not required on this OS. Use 'xdg-mime' if desired.");

    Ok(())
}

#[cfg(target_os = "windows")]
fn cmd_uninstall(dry: bool) -> Result<()> {
    if dry {
        println!("Would remove per-user association for .tgsk");
        println!("  - Delete HKCU/Software/Classes/.tgsk");
        println!("  - Delete HKCU/Software/Classes/TagSpeakFile");

        return Ok(());
    }

    use winreg::RegKey;

    use winreg::enums::{HKEY_CURRENT_USER, KEY_ALL_ACCESS};

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);

    if let Ok(classes) = hkcu.open_subkey_with_flags("Software\\Classes", KEY_ALL_ACCESS) {
        let _ = classes.delete_subkey_all(".tgsk");

        let _ = classes.delete_subkey_all("TagSpeakFile");
    }

    if let Ok(file_exts) = hkcu.open_subkey_with_flags(
        "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\FileExts",
        KEY_ALL_ACCESS,
    ) {
        let _ = file_exts.delete_subkey_all(".tgsk");
    }

    refresh_icons();

    println!("Removed per-user association for .tgsk");

    Ok(())
}

#[cfg(not(target_os = "windows"))]

fn cmd_uninstall(_dry: bool) -> Result<()> {
    println!("Nothing to uninstall on this OS.");

    Ok(())
}

#[cfg(target_os = "windows")]
fn refresh_icons() {
    use windows_sys::Win32::UI::Shell::{SHCNE_ASSOCCHANGED, SHCNF_IDLIST, SHChangeNotify};

    unsafe {
        SHChangeNotify(
            SHCNE_ASSOCCHANGED as i32,
            SHCNF_IDLIST,
            std::ptr::null(),
            std::ptr::null(),
        );
    }
}

#[cfg(not(target_os = "windows"))]

fn refresh_icons() {}
