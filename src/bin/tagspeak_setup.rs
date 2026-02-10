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

    /// Force the GUI wizard (Windows only; requires `setup_wizard` feature)
    #[arg(long, action = ArgAction::SetTrue)]
    gui: bool,

    /// Force the CLI wizard even if GUI is available
    #[arg(long, action = ArgAction::SetTrue)]
    cli: bool,

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
        #[cfg(feature = "setup_wizard")]
        {
            let has_direct_cli = opts.check
                || opts.associate.is_some()
                || opts.uninstall
                || opts.associate_dry
                || opts.uninstall_dry
                || opts.cli;

            if opts.gui {
                return gui::launch();
            }
            if !has_direct_cli {
                return gui::launch();
            }
        }

        #[cfg(not(feature = "setup_wizard"))]
        {
            if opts.gui {
                eprintln!("GUI wizard requires `--features setup_wizard`; falling back to CLI.");
            }
        }

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

    let result = perform_install(&repo_root, &target_dir, |msg| println!("{msg}"))?;

    println!("Installed engine to {}", result.engine_path.display());
    if result.path_updated {
        println!("Restart open terminals to pick up the updated PATH.");
    }
    println!("Install complete.");
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

    let result = perform_uninstall(&install_dir, |msg| println!("{msg}"))?;
    if result.path_updated {
        println!("Restart open terminals to pick up the updated PATH.");
    }
    println!("Uninstall complete.");
    Ok(())
}

#[cfg(target_os = "windows")]
struct InstallResult {
    engine_path: PathBuf,
    path_updated: bool,
}

#[cfg(target_os = "windows")]
struct UninstallResult {
    path_updated: bool,
}

#[cfg(target_os = "windows")]
fn perform_install<F>(repo_root: &Path, target_dir: &Path, mut progress: F) -> Result<InstallResult>
where
    F: FnMut(&str),
{
    progress("Creating install directory...");
    fs::create_dir_all(target_dir)?;
    let target_dir = match fs::canonicalize(target_dir) {
        Ok(path) => path,
        Err(_) => target_dir.to_path_buf(),
    };
    let engine_path = target_dir.join("tagspeak_rs.exe");

    progress("Building TagSpeak engine (cargo build --release)...");
    let status = Command::new("cargo")
        .args(["build", "--release"])
        .current_dir(repo_root)
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

    progress("Copying engine to install directory...");
    fs::copy(&built_engine, &engine_path)
        .map_err(|err| anyhow!("Failed to copy engine to {}: {err}", engine_path.display()))?;

    let path_changed = ensure_path_entry(&target_dir)?;
    if path_changed {
        broadcast_environment_change();
        progress(&format!(
            "Added {} to your user PATH.",
            target_dir.display()
        ));
    } else {
        progress(&format!(
            "{} is already on your user PATH.",
            target_dir.display()
        ));
    }

    store_install_dir(&target_dir)?;
    progress("Associating .tgsk with TagSpeak...");
    cmd_associate(&engine_path, false)?;

    Ok(InstallResult {
        engine_path,
        path_updated: path_changed,
    })
}

#[cfg(target_os = "windows")]
fn perform_uninstall<F>(install_dir: &Path, mut progress: F) -> Result<UninstallResult>
where
    F: FnMut(&str),
{
    if install_dir.exists() {
        progress(&format!("Removing {}", install_dir.display()));
        fs::remove_dir_all(install_dir).map_err(|err| {
            anyhow!(
                "Failed to remove install directory {}: {err}",
                install_dir.display()
            )
        })?;
    } else {
        progress(&format!(
            "Install directory {} does not exist, skipping removal.",
            install_dir.display()
        ));
    }

    let path_changed = remove_path_entry(install_dir)?;
    if path_changed {
        broadcast_environment_change();
        progress(&format!(
            "Removed {} from your user PATH.",
            install_dir.display()
        ));
    }

    progress("Removing file associations...");
    cmd_uninstall(false)?;
    clear_install_dir_record()?;
    Ok(UninstallResult {
        path_updated: path_changed,
    })
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
fn current_association() -> Result<Option<String>> {
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
        Ok(cmd)
    } else {
        Ok(None)
    }
}

#[cfg(target_os = "windows")]
fn cmd_check() -> Result<()> {
    match current_association()? {
        Some(cmd) => println!("Current handler: {cmd}"),
        None => println!("No per-user association found."),
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

#[cfg(all(target_os = "windows", feature = "setup_wizard"))]
mod gui {
    use super::*;
    use native_windows_gui as nwg;
    use std::cell::RefCell;
    use std::path::{Path, PathBuf};
    use std::rc::Rc;
    use std::sync::mpsc::{self, Receiver};
    use std::thread;

    const WINDOW_W: i32 = 720;
    const WINDOW_H: i32 = 520;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Action {
        Install,
        Uninstall,
        Check,
    }

    enum WorkerEvent {
        Log(String),
        Done(Result<(), String>),
    }

    pub fn launch() -> Result<()> {
        nwg::init().map_err(|e| anyhow!("Failed to initialise GUI: {e}"))?;
        nwg::Font::set_global_family("Segoe UI").ok();

        let ui = Rc::new(RefCell::new(Wizard::new()?));

        let handler = {
            let ui = Rc::clone(&ui);
            let handle = ui.borrow().window.handle;
            nwg::full_bind_event_handler(&handle, move |evt, _evt_data, handle| {
                let mut ui = ui.borrow_mut();
                ui.handle_event(evt, handle);
            })
        };

        nwg::dispatch_thread_events();
        nwg::unbind_event_handler(&handler);
        Ok(())
    }

    struct Wizard {
        window: nwg::Window,
        notice: nwg::Notice,
        page_intro: nwg::Frame,
        page_paths: nwg::Frame,
        page_summary: nwg::Frame,
        page_progress: nwg::Frame,
        _intro_title: nwg::Label,
        _intro_body: nwg::Label,
        action_install: nwg::RadioButton,
        action_uninstall: nwg::RadioButton,
        action_check: nwg::RadioButton,
        repo_label: nwg::Label,
        repo_input: nwg::TextInput,
        repo_browse: nwg::Button,
        repo_hint: nwg::Label,
        install_label: nwg::Label,
        install_input: nwg::TextInput,
        install_browse: nwg::Button,
        install_hint: nwg::Label,
        summary_box: nwg::RichTextBox,
        progress_log: nwg::RichTextBox,
        progress_bar: nwg::ProgressBar,
        status: nwg::Label,
        back_btn: nwg::Button,
        next_btn: nwg::Button,
        cancel_btn: nwg::Button,
        page_index: usize,
        worker_rx: Option<Receiver<WorkerEvent>>,
        worker_running: bool,
        log_count: u32,
    }

    impl Wizard {
        fn new() -> Result<Self> {
            let mut window = nwg::Window::default();
            nwg::Window::builder()
                .size((WINDOW_W, WINDOW_H))
                .position((150, 150))
                .title("TagSpeak Setup Wizard")
                .build(&mut window)?;

            let mut notice = nwg::Notice::default();
            nwg::Notice::builder().parent(&window).build(&mut notice)?;

            let page_size = (WINDOW_W - 20, WINDOW_H - 110);

            let mut page_intro = nwg::Frame::default();
            nwg::Frame::builder()
                .parent(&window)
                .size(page_size)
                .position((10, 10))
                .build(&mut page_intro)?;

            let mut page_paths = nwg::Frame::default();
            nwg::Frame::builder()
                .parent(&window)
                .size(page_size)
                .position((10, 10))
                .build(&mut page_paths)?;

            let mut page_summary = nwg::Frame::default();
            nwg::Frame::builder()
                .parent(&window)
                .size(page_size)
                .position((10, 10))
                .build(&mut page_summary)?;

            let mut page_progress = nwg::Frame::default();
            nwg::Frame::builder()
                .parent(&window)
                .size(page_size)
                .position((10, 10))
                .build(&mut page_progress)?;

            // Intro content
            let mut intro_title = nwg::Label::default();
            nwg::Label::builder()
                .parent(&page_intro)
                .text("Welcome to TagSpeak")
                .position((16, 12))
                .size((page_size.0 - 30, 24))
                .build(&mut intro_title)?;

            let mut intro_body = nwg::Label::default();
            nwg::Label::builder()
                .parent(&page_intro)
                .text("Choose what you want to do. The wizard will handle builds, PATH updates, and .tgsk file associations.")
                .position((16, 40))
                .size((page_size.0 - 30, 40))
                .build(&mut intro_body)?;

            let mut action_install = nwg::RadioButton::default();
            nwg::RadioButton::builder()
                .parent(&page_intro)
                .text("Install / update TagSpeak (build engine, add to PATH, associate .tgsk)")
                .position((24, 100))
                .size((page_size.0 - 60, 26))
                .flags(nwg::RadioButtonFlags::VISIBLE | nwg::RadioButtonFlags::GROUP)
                .check_state(nwg::RadioButtonState::Checked)
                .build(&mut action_install)?;

            let mut action_uninstall = nwg::RadioButton::default();
            nwg::RadioButton::builder()
                .parent(&page_intro)
                .text("Uninstall TagSpeak (remove engine, PATH entry, associations)")
                .position((24, 130))
                .size((page_size.0 - 60, 26))
                .build(&mut action_uninstall)?;

            let mut action_check = nwg::RadioButton::default();
            nwg::RadioButton::builder()
                .parent(&page_intro)
                .text("Show current .tgsk association only")
                .position((24, 160))
                .size((page_size.0 - 60, 26))
                .build(&mut action_check)?;

            // Paths page
            let mut repo_label = nwg::Label::default();
            nwg::Label::builder()
                .parent(&page_paths)
                .text("TagSpeak repository (folder with Cargo.toml)")
                .position((16, 12))
                .size((page_size.0 - 30, 20))
                .build(&mut repo_label)?;

            let mut repo_input = nwg::TextInput::default();
            nwg::TextInput::builder()
                .parent(&page_paths)
                .size((page_size.0 - 170, 26))
                .position((16, 36))
                .build(&mut repo_input)?;

            let mut repo_browse = nwg::Button::default();
            nwg::Button::builder()
                .parent(&page_paths)
                .text("Browse…")
                .size((120, 26))
                .position((page_size.0 - 136, 36))
                .build(&mut repo_browse)?;

            let mut repo_hint = nwg::Label::default();
            nwg::Label::builder()
                .parent(&page_paths)
                .text("Leave empty to auto-detect from the current folder.")
                .position((16, 64))
                .size((page_size.0 - 30, 20))
                .build(&mut repo_hint)?;

            let mut install_label = nwg::Label::default();
            nwg::Label::builder()
                .parent(&page_paths)
                .text("Install location (where tagspeak_rs.exe is copied)")
                .position((16, 104))
                .size((page_size.0 - 30, 20))
                .build(&mut install_label)?;

            let mut install_input = nwg::TextInput::default();
            nwg::TextInput::builder()
                .parent(&page_paths)
                .size((page_size.0 - 170, 26))
                .position((16, 128))
                .build(&mut install_input)?;

            let mut install_browse = nwg::Button::default();
            nwg::Button::builder()
                .parent(&page_paths)
                .text("Browse…")
                .size((120, 26))
                .position((page_size.0 - 136, 128))
                .build(&mut install_browse)?;

            let mut install_hint = nwg::Label::default();
            nwg::Label::builder()
                .parent(&page_paths)
                .text("Default: %LOCALAPPDATA%\\TagSpeak")
                .position((16, 156))
                .size((page_size.0 - 30, 20))
                .build(&mut install_hint)?;

            // Summary
            let mut summary_box = nwg::RichTextBox::default();
            nwg::RichTextBox::builder()
                .parent(&page_summary)
                .readonly(true)
                .flags(
                    nwg::RichTextBoxFlags::VISIBLE
                        | nwg::RichTextBoxFlags::AUTOVSCROLL
                        | nwg::RichTextBoxFlags::VSCROLL,
                )
                .limit(128000)
                .size((page_size.0 - 20, page_size.1 - 40))
                .position((10, 10))
                .build(&mut summary_box)?;

            // Progress
            let mut progress_log = nwg::RichTextBox::default();
            nwg::RichTextBox::builder()
                .parent(&page_progress)
                .readonly(true)
                .flags(
                    nwg::RichTextBoxFlags::VISIBLE
                        | nwg::RichTextBoxFlags::AUTOVSCROLL
                        | nwg::RichTextBoxFlags::VSCROLL,
                )
                .limit(256000)
                .size((page_size.0 - 20, page_size.1 - 70))
                .position((10, 10))
                .build(&mut progress_log)?;

            let mut progress_bar = nwg::ProgressBar::default();
            nwg::ProgressBar::builder()
                .parent(&page_progress)
                .range(0..100)
                .position((10, page_size.1 - 50))
                .size((page_size.0 - 20, 24))
                .build(&mut progress_bar)?;

            // Nav + status
            let mut status = nwg::Label::default();
            nwg::Label::builder()
                .parent(&window)
                .text("Ready.")
                .position((16, WINDOW_H - 60))
                .size((WINDOW_W - 320, 24))
                .build(&mut status)?;

            let mut back_btn = nwg::Button::default();
            nwg::Button::builder()
                .parent(&window)
                .text("< Back")
                .size((90, 28))
                .position((WINDOW_W - 320, WINDOW_H - 68))
                .build(&mut back_btn)?;
            back_btn.set_enabled(false);

            let mut next_btn = nwg::Button::default();
            nwg::Button::builder()
                .parent(&window)
                .text("Next >")
                .size((110, 28))
                .position((WINDOW_W - 220, WINDOW_H - 68))
                .build(&mut next_btn)?;

            let mut cancel_btn = nwg::Button::default();
            nwg::Button::builder()
                .parent(&window)
                .text("Cancel")
                .size((80, 28))
                .position((WINDOW_W - 100, WINDOW_H - 68))
                .build(&mut cancel_btn)?;

            // Prefill paths
            if let Ok(root) = detect_repo_root() {
                repo_input.set_text(&root.display().to_string());
            }
            if let Some(default_dir) = default_install_guess() {
                install_input.set_text(&default_dir.display().to_string());
            }

            // Hide all but intro
            page_paths.set_visible(false);
            page_summary.set_visible(false);
            page_progress.set_visible(false);

            Ok(Self {
                window,
                notice,
                page_intro,
                page_paths,
                page_summary,
                page_progress,
                _intro_title: intro_title,
                _intro_body: intro_body,
                action_install,
                action_uninstall,
                action_check,
                repo_label,
                repo_input,
                repo_browse,
                repo_hint,
                install_label,
                install_input,
                install_browse,
                install_hint,
                summary_box,
                progress_log,
                progress_bar,
                status,
                back_btn,
                next_btn,
                cancel_btn,
                page_index: 0,
                worker_rx: None,
                worker_running: false,
                log_count: 0,
            })
        }

        fn handle_event(&mut self, evt: nwg::Event, handle: nwg::ControlHandle) {
            match evt {
                nwg::Event::OnWindowClose => self.on_cancel(),
                nwg::Event::OnButtonClick => {
                    if handle == self.next_btn.handle {
                        self.on_next();
                    } else if handle == self.back_btn.handle {
                        self.on_back();
                    } else if handle == self.cancel_btn.handle {
                        self.on_cancel();
                    } else if handle == self.repo_browse.handle {
                        self.on_browse_repo();
                    } else if handle == self.install_browse.handle {
                        self.on_browse_install();
                    } else if handle == self.action_install.handle
                        || handle == self.action_uninstall.handle
                        || handle == self.action_check.handle
                    {
                        self.on_action_changed();
                    }
                }
                nwg::Event::OnNotice if handle == self.notice.handle => {
                    self.process_worker_messages();
                }
                _ => {}
            }
        }

        fn on_next(&mut self) {
            if self.worker_running {
                return;
            }
            match self.page_index {
                0 => {
                    self.page_index = 1;
                    self.refresh_paths_visibility();
                }
                1 => {
                    let action = self.current_action();
                    if action == Action::Check {
                        self.page_index = 2;
                        self.populate_summary();
                    } else if self.validate_inputs() {
                        self.page_index = 2;
                        self.populate_summary();
                    } else {
                        return;
                    }
                }
                2 => {
                    if self.current_action() == Action::Check || self.validate_inputs() {
                        self.start_worker();
                    } else {
                        return;
                    }
                }
                3 => {
                    if !self.worker_running {
                        nwg::stop_thread_dispatch();
                    }
                }
                _ => {}
            }
            self.show_page(self.page_index);
            self.sync_nav();
        }

        fn on_back(&mut self) {
            if self.worker_running {
                return;
            }
            if self.page_index > 0 {
                self.page_index -= 1;
                self.show_page(self.page_index);
                self.sync_nav();
            }
        }

        fn on_cancel(&mut self) {
            if self.worker_running {
                return;
            }
            nwg::stop_thread_dispatch();
        }

        fn on_action_changed(&mut self) {
            if self.current_action() == Action::Uninstall {
                if let Ok(Some(saved)) = read_install_dir() {
                    self.install_input.set_text(&saved.display().to_string());
                }
            } else if self.install_input.text().trim().is_empty() {
                if let Some(default_dir) = default_install_guess() {
                    self.install_input
                        .set_text(&default_dir.display().to_string());
                }
            }
            self.refresh_paths_visibility();
            self.sync_nav();
        }

        fn on_browse_repo(&mut self) {
            let start = self.repo_path();
            if let Some(path) =
                select_directory(&self.window, "Select TagSpeak repository", start.as_deref())
            {
                self.repo_input.set_text(&path.display().to_string());
            }
        }

        fn on_browse_install(&mut self) {
            let start = self.install_path();
            if let Some(path) =
                select_directory(&self.window, "Select install folder", start.as_deref())
            {
                self.install_input.set_text(&path.display().to_string());
            }
        }

        fn show_page(&mut self, page: usize) {
            self.page_intro.set_visible(page == 0);
            self.page_paths.set_visible(page == 1);
            self.page_summary.set_visible(page == 2);
            self.page_progress.set_visible(page == 3);
        }

        fn sync_nav(&mut self) {
            self.back_btn
                .set_enabled(self.page_index > 0 && !self.worker_running);
            self.cancel_btn.set_enabled(!self.worker_running);
            let label = match self.page_index {
                0 | 1 => "Next >",
                2 => match self.current_action() {
                    Action::Install => "Install",
                    Action::Uninstall => "Uninstall",
                    Action::Check => "Show",
                },
                _ => "Close",
            };
            self.next_btn.set_text(label);
            self.next_btn
                .set_enabled(!self.worker_running || self.page_index == 3);
        }

        fn current_action(&self) -> Action {
            if self.action_uninstall.check_state() == nwg::RadioButtonState::Checked {
                Action::Uninstall
            } else if self.action_check.check_state() == nwg::RadioButtonState::Checked {
                Action::Check
            } else {
                Action::Install
            }
        }

        fn repo_path(&self) -> Option<PathBuf> {
            let value = self.repo_input.text();
            let trimmed = value.trim();
            if trimmed.is_empty() {
                detect_repo_root().ok()
            } else {
                Some(PathBuf::from(trimmed))
            }
        }

        fn install_path(&self) -> Option<PathBuf> {
            let value = self.install_input.text();
            let trimmed = value.trim();
            if trimmed.is_empty() {
                default_install_guess()
            } else {
                Some(PathBuf::from(trimmed))
            }
        }

        fn refresh_paths_visibility(&mut self) {
            let needs_repo = matches!(self.current_action(), Action::Install);
            self.repo_label.set_visible(needs_repo);
            self.repo_input.set_visible(needs_repo);
            self.repo_browse.set_visible(needs_repo);
            self.repo_hint.set_visible(needs_repo);

            let needs_install_dir =
                matches!(self.current_action(), Action::Install | Action::Uninstall);
            self.install_label.set_visible(needs_install_dir);
            self.install_input.set_visible(needs_install_dir);
            self.install_browse.set_visible(needs_install_dir);
            self.install_hint.set_visible(needs_install_dir);
        }

        fn validate_inputs(&mut self) -> bool {
            match self.current_action() {
                Action::Install => {
                    let Some(repo) = self.repo_path() else {
                        self.set_status("Select the TagSpeak repository (Cargo.toml).");
                        return false;
                    };
                    if !repo.join("Cargo.toml").exists() {
                        self.set_status("Cargo.toml not found in the selected repository.");
                        return false;
                    }
                    if self.install_path().is_none() {
                        self.set_status("Choose an install directory.");
                        return false;
                    }
                }
                Action::Uninstall => {
                    if self.install_path().is_none() {
                        self.set_status("Choose the install directory to remove.");
                        return false;
                    }
                }
                Action::Check => {}
            }
            self.set_status("Ready.");
            true
        }

        fn populate_summary(&mut self) {
            let mut summary = String::new();
            match self.current_action() {
                Action::Install => {
                    let repo = self
                        .repo_path()
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|| "<choose a repository>".into());
                    let install = self
                        .install_path()
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|| "<choose an install directory>".into());
                    summary.push_str("Action: Install / Update TagSpeak\n\n");
                    summary.push_str("Source repo:\n  ");
                    summary.push_str(&repo);
                    summary.push_str("\n\nInstall to:\n  ");
                    summary.push_str(&install);
                    summary.push_str("\n\nSteps:\n  • Build engine (cargo build --release)\n  • Copy tagspeak_rs.exe\n  • Add folder to user PATH\n  • Associate .tgsk files");
                }
                Action::Uninstall => {
                    let install = self
                        .install_path()
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|| "<choose an install directory>".into());
                    summary.push_str("Action: Uninstall TagSpeak\n\n");
                    summary.push_str("Install folder to remove:\n  ");
                    summary.push_str(&install);
                    summary.push_str("\n\nSteps:\n  • Delete install folder\n  • Remove folder from user PATH\n  • Remove .tgsk association");
                }
                Action::Check => {
                    summary
                        .push_str("Action: Show current .tgsk handler\n\nNo changes will be made.");
                }
            }
            self.summary_box.set_text(&summary);
            self.summary_box.set_readonly(true);
        }

        fn start_worker(&mut self) {
            self.reset_progress();
            self.page_index = 3;
            self.show_page(3);
            self.sync_nav();
            self.worker_running = true;
            let (tx, rx) = mpsc::channel();
            self.worker_rx = Some(rx);
            let sender = self.notice.sender();
            let action = self.current_action();
            let repo = self.repo_path();
            let install_dir = self.install_path();

            thread::spawn(move || {
                let push = |msg: &str| {
                    let _ = tx.send(WorkerEvent::Log(msg.to_string()));
                    sender.notice();
                };

                let result = match action {
                    Action::Install => match (repo, install_dir) {
                        (Some(repo_root), Some(target_dir)) => {
                            push("Starting install...");
                            perform_install(&repo_root, &target_dir, |m| push(m)).map(|_| ())
                        }
                        _ => Err(anyhow!("Missing repo or install path")),
                    },
                    Action::Uninstall => {
                        if let Some(dir) = install_dir {
                            push("Starting uninstall...");
                            perform_uninstall(&dir, |m| push(m)).map(|_| ())
                        } else {
                            Err(anyhow!("Missing install path"))
                        }
                    }
                    Action::Check => match current_association() {
                        Ok(Some(cmd)) => {
                            push(&format!("Current handler: {cmd}"));
                            Ok(())
                        }
                        Ok(None) => {
                            push("No per-user association found.");
                            Ok(())
                        }
                        Err(e) => Err(e),
                    },
                };

                let done = result.map_err(|e| format!("{e:#}"));
                let _ = tx.send(WorkerEvent::Done(done));
                sender.notice();
            });
        }

        fn process_worker_messages(&mut self) {
            let pending: Vec<WorkerEvent> = if let Some(rx) = self.worker_rx.as_ref() {
                let mut items = Vec::new();
                while let Ok(msg) = rx.try_recv() {
                    items.push(msg);
                }
                items
            } else {
                Vec::new()
            };

            for msg in pending {
                match msg {
                    WorkerEvent::Log(line) => self.append_log(&line),
                    WorkerEvent::Done(res) => {
                        self.worker_running = false;
                        match res {
                            Ok(()) => self.set_status("Completed."),
                            Err(err) => {
                                self.append_log(&format!("Error: {err}"));
                                self.set_status("Failed.");
                            }
                        }
                        self.sync_nav();
                    }
                }
            }
        }

        fn append_log(&mut self, line: &str) {
            self.progress_log.appendln(line);
            self.log_count = self.log_count.saturating_add(1);
            let pos = (self.log_count % 100).max(1);
            self.progress_bar.set_pos(pos);
        }

        fn set_status(&mut self, msg: &str) {
            self.status.set_text(msg);
        }

        fn reset_progress(&mut self) {
            self.progress_log.set_text("");
            self.progress_bar.set_pos(0);
            self.log_count = 0;
            self.set_status("Running...");
        }
    }

    fn select_directory(
        parent: &nwg::Window,
        title: &str,
        start: Option<&Path>,
    ) -> Option<PathBuf> {
        let mut dialog = nwg::FileDialog::default();
        let mut builder = nwg::FileDialog::builder()
            .title(title)
            .action(nwg::FileDialogAction::OpenDirectory);
        if let Some(path) = start {
            builder = builder.default_folder(path.display().to_string());
        }
        if builder.build(&mut dialog).is_ok() && dialog.run(Some(parent)) {
            if let Ok(sel) = dialog.get_selected_item() {
                return Some(PathBuf::from(sel));
            }
        }
        None
    }

    fn default_install_guess() -> Option<PathBuf> {
        let repo =
            detect_repo_root().unwrap_or_else(|_| std::env::current_dir().unwrap_or_default());
        default_install_dir(&repo).ok()
    }
}
