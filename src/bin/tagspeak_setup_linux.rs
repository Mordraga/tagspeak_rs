#[cfg(target_os = "linux")]
fn main() {
    // Minimal, per-user, opt-in setup helper.
    // Defaults to help text; performs actions only when flags are provided.
    use std::env;
    let mut args = env::args().skip(1);
    let cmd = args.next();
    match cmd.as_deref() {
        None | Some("--help") => {
            print_help();
        }
        Some("--check") => {
            let _ = check_association();
        }
        Some("--associate") => {
            let engine = args.next();
            if let Err(e) = associate(engine.as_deref(), /*dry=*/false) {
                eprintln!("Error: {e:#}");
                std::process::exit(1);
            }
        }
        Some("--associate-dry") | Some("--print") => {
            let engine = args.next();
            if let Err(e) = associate(engine.as_deref(), /*dry=*/true) {
                eprintln!("Error: {e:#}");
                std::process::exit(1);
            }
        }
        Some("--uninstall") => {
            if let Err(e) = uninstall(/*dry=*/false) {
                eprintln!("Error: {e:#}");
                std::process::exit(1);
            }
        }
        Some("--uninstall-dry") => {
            if let Err(e) = uninstall(/*dry=*/true) {
                eprintln!("Error: {e:#}");
                std::process::exit(1);
            }
        }
        _ => {
            print_help();
        }
    }
}

#[cfg(target_os = "linux")]
fn print_help() {
    println!(
        "TagSpeak Setup (Linux)\n\
         \n\
         Usage:\n\
           tagspeak_setup_linux --help                 Show this help\n\
           tagspeak_setup_linux --check                Show current handler for text/x-tagspeak\n\
           tagspeak_setup_linux --associate [ENGINE]  Associate .tgsk with ENGINE (per-user)\n\
           tagspeak_setup_linux --associate-dry [ENGINE]  Print steps but do nothing\n\
           tagspeak_setup_linux --uninstall           Remove per-user association\n\
           tagspeak_setup_linux --uninstall-dry       Print steps but do nothing\n\
         \n\
         Notes:\n\
           • ENGINE defaults to `tagspeak_rs` on PATH if omitted.\n\
           • All changes are per-user (~/.local/*). No root needed.\n\
        "
    );
}

#[cfg(target_os = "linux")]
fn associate(engine_arg: Option<&str>, dry: bool) -> anyhow::Result<()> {
    use anyhow::{bail, Context};
    use std::fs;
    use std::path::PathBuf;
    let engine = match engine_arg {
        Some(p) => PathBuf::from(p),
        None => which::which("tagspeak_rs").context("tagspeak_rs not found on PATH; pass ENGINE path")?,
    };
    if !engine.exists() {
        bail!("ENGINE not found: {}", engine.display());
    }

    let home = dirs::home_dir().context("no home dir")?;
    let apps = home.join(".local/share/applications");
    let mime_pkgs = home.join(".local/share/mime/packages");
    let desktop = apps.join("tagspeak.desktop");
    let mime_xml = mime_pkgs.join("tagspeak.xml");

    let desktop_content = format!(
        "[Desktop Entry]\nName=TagSpeak\nExec=\"{}\" %f\nType=Application\nMimeType=text/x-tagspeak;\nNoDisplay=false\nTerminal=false\n",
        engine.display()
    );
    let mime_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<mime-info xmlns="http://www.freedesktop.org/standards/shared-mime-info">
  <mime-type type="text/x-tagspeak">
    <comment>TagSpeak script</comment>
    <glob pattern="*.tgsk"/>
  </mime-type>
</mime-info>
"#;

    if dry {
        println!("Would create: {}\n{}", desktop.display(), desktop_content);
        println!("Would create: {}\n{}", mime_xml.display(), mime_content);
    } else {
        fs::create_dir_all(&apps)?;
        fs::create_dir_all(&mime_pkgs)?;
        fs::write(&desktop, desktop_content)?;
        fs::write(&mime_xml, mime_content)?;
    }

    // Update databases and set default
    let mime_root = mime_pkgs.parent().unwrap().to_path_buf();
    run_cmd("update-desktop-database", &[apps.to_string_lossy().as_ref()], dry);
    run_cmd("update-mime-database", &[mime_root.to_string_lossy().as_ref()], dry);
    run_cmd(
        "xdg-mime",
        &["default", "tagspeak.desktop", "text/x-tagspeak"],
        dry,
    );
    println!("Associated .tgsk -> {} (per-user)", engine.display());
    Ok(())
}

#[cfg(target_os = "linux")]
fn uninstall(dry: bool) -> anyhow::Result<()> {
    use std::fs;
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("no home dir"))?;
    let apps = home.join(".local/share/applications");
    let mime_pkgs = home.join(".local/share/mime/packages");
    let desktop = apps.join("tagspeak.desktop");
    let mime_xml = mime_pkgs.join("tagspeak.xml");
    if dry {
        println!("Would remove: {}", desktop.display());
        println!("Would remove: {}", mime_xml.display());
    } else {
        let _ = fs::remove_file(&desktop);
        let _ = fs::remove_file(&mime_xml);
    }
    let mime_root = mime_pkgs.parent().unwrap().to_path_buf();
    run_cmd("update-desktop-database", &[apps.to_string_lossy().as_ref()], dry);
    run_cmd("update-mime-database", &[mime_root.to_string_lossy().as_ref()], dry);
    println!("Removed per-user .tgsk association (if present)");
    Ok(())
}

#[cfg(target_os = "linux")]
fn check_association() -> anyhow::Result<()> {
    run_cmd("xdg-mime", &["query", "default", "text/x-tagspeak"], false);
    Ok(())
}

#[cfg(target_os = "linux")]
fn run_cmd(cmd: &str, args: &[&str], dry: bool) {
    use std::process::Command;
    if dry {
        eprintln!("$ {} {}", cmd, args.join(" "));
        return;
    }
    match Command::new(cmd).args(args).status() {
        Ok(s) if s.success() => {}
        Ok(s) => eprintln!("{cmd} exited with code {:?}", s.code()),
        Err(e) => eprintln!("could not run {cmd}: {e}"),
    }
}

#[cfg(not(target_os = "linux"))]
fn main() {}
