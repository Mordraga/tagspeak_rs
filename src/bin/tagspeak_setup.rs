use anyhow::{anyhow, Result};
use clap::{ArgAction, Parser};
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(name = "tagspeak_setup", about = "Terminal setup for TagSpeak file associations")]
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

#[cfg(target_os = "windows")]
fn cmd_check() -> Result<()> {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let classes = hkcu.open_subkey("Software\\Classes").ok();
    let prog = classes
        .as_ref()
        .and_then(|c| c.open_subkey("TagSpeakFile").ok());
    if let Some(prog) = prog {
        let cmd_key = prog.open_subkey("shell\\open\\command").ok();
        let cmd: Option<String> = cmd_key
            .as_ref()
            .and_then(|k| k.get_value("").ok());
        println!("Current handler: {}", cmd.unwrap_or_else(|| "<not set>".into()));
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
        println!("Would set per-user association for .tgsk → {engine_str}");
        println!("  - HKCU/Software/Classes/.tgsk → TagSpeakFile");
        println!("  - HKCU/Software/Classes/TagSpeakFile/shell/open/command → {command}");
        return Ok(());
    }

    use winreg::enums::{HKEY_CURRENT_USER, KEY_ALL_ACCESS};
    use winreg::RegKey;

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
    use winreg::enums::{HKEY_CURRENT_USER, KEY_ALL_ACCESS};
    use winreg::RegKey;
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
    use windows_sys::Win32::UI::Shell::{SHChangeNotify, SHCNE_ASSOCCHANGED, SHCNF_IDLIST};
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

