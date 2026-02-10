use anyhow::{Result, bail};
use std::io::{self, Write};

use crate::kernel::{Arg, Packet, Runtime, Value};

fn is_noninteractive() -> bool {
    std::env::var("TAGSPEAK_NONINTERACTIVE")
        .map(|v| matches!(v.as_str(), "1" | "true" | "yes" | "y"))
        .unwrap_or(false)
}

fn resolve_options(rt: &Runtime, p: &Packet) -> Result<Vec<String>> {
    // Options provided as @"opt1|opt2|opt3" or @ident resolving to string with '|'
    // If no arg, try last value if it's a string with separators.
    if let Some(arg) = &p.arg {
        match arg {
            Arg::Str(s) => {
                let s = s.trim_matches('"').to_string();
                return Ok(split_opts(&s));
            }
            Arg::Ident(id) => {
                if let Some(Value::Str(s)) = rt.get_var(id) {
                    return Ok(split_opts(&s));
                }
            }
            _ => {}
        }
    }
    match &rt.last {
        Value::Str(s) => Ok(split_opts(s)),
        _ => Ok(Vec::new()),
    }
}

fn split_opts(s: &str) -> Vec<String> {
    s.split('|')
        .map(|t| t.trim())
        .filter(|t| !t.is_empty())
        .map(|t| t.to_string())
        .collect()
}

pub fn handle(rt: &mut Runtime, p: &Packet) -> Result<Value> {
    let opts = resolve_options(rt, p)?;
    if opts.is_empty() {
        bail!(r#"ui:select requires @"a|b|c" or string input with options"#);
    }

    if !is_noninteractive() {
        // Try GUI first when compiled with egui
        #[cfg(feature = "ui_egui")]
        {
            if let Some(sel) = run_select_gui(&opts)? {
                return Ok(Value::Str(sel));
            } else {
                return Ok(Value::Unit);
            }
        }

        // Console fallback (no ui_egui)
        #[cfg(not(feature = "ui_egui"))]
        {
            println!("[UI] Select an option:");
            for (i, opt) in opts.iter().enumerate() {
                println!("  {}. {}", i + 1, opt);
            }

            let mut stdout = io::stdout();
            loop {
                write!(stdout, "Enter choice [1-{}] or 'q' to cancel: ", opts.len())?;
                stdout.flush()?;

                let mut line = String::new();
                io::stdin().read_line(&mut line).ok();
                let line = line.trim();
                if line.eq_ignore_ascii_case("q") || line.is_empty() {
                    return Ok(Value::Unit);
                }
                if let Ok(n) = line.parse::<usize>() {
                    if n >= 1 && n <= opts.len() {
                        return Ok(Value::Str(opts[n - 1].clone()));
                    }
                }
                println!("Invalid selection. Try again.");
            }
        }
    }

    // Noninteractive: default to Unit
    Ok(Value::Unit)
}

#[cfg(feature = "ui_egui")]
fn run_select_gui(options: &[String]) -> Result<Option<String>> {
    use eframe::{NativeOptions, egui};
    use std::sync::{Arc, Mutex};

    struct SelectApp {
        opts: Vec<String>,
        idx: usize,
        chosen: Arc<Mutex<Option<String>>>,
    }
    impl eframe::App for SelectApp {
        fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.heading("Select an option");
                ui.separator();
                for i in 0..self.opts.len() {
                    ui.radio_value(&mut self.idx, i, self.opts[i].as_str());
                }
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("OK").clicked() {
                        let mut g = self.chosen.lock().unwrap();
                        *g = Some(self.opts[self.idx].clone());
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    if ui.button("Cancel").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
            });
        }
    }

    let chosen = Arc::new(Mutex::new(None));
    let chosen_clone = chosen.clone();
    let opts_vec = options.to_vec();

    let app = SelectApp {
        opts: opts_vec,
        idx: 0,
        chosen: chosen_clone,
    };
    let _ = eframe::run_native(
        "TagSpeak Select",
        NativeOptions::default(),
        Box::new(|_cc| Box::new(app)),
    );
    Ok(chosen.lock().unwrap().clone())
}
