use crate::kernel::{Packet, Runtime, Value};
use anyhow::Result;

fn is_noninteractive() -> bool {
    std::env::var("TAGSPEAK_NONINTERACTIVE")
        .map(|v| matches!(v.as_str(), "1" | "true" | "yes" | "y"))
        .unwrap_or(false)
}

// [ui:alert@"message"] -> prints the message; returns prior value unchanged
// If no @arg, prints the pretty form of the prior value.
// Respects TAGSPEAK_NONINTERACTIVE by still printing (non-destructive) and passing through.
pub fn handle(rt: &mut Runtime, p: &Packet) -> Result<Value> {
    let prior = rt.last.clone();
    let msg = match p.arg.as_ref() {
        Some(arg) => match rt.resolve_arg(arg)? {
            Value::Str(s) => s,
            Value::Num(n) => format!("{}", n),
            Value::Bool(b) => format!("{}", b),
            Value::Doc(_) => String::from("<doc>"),
            Value::Unit => String::from("()"),
        },
        None => match &prior {
            Value::Str(s) => s.clone(),
            Value::Num(n) => format!("{}", n),
            Value::Bool(b) => format!("{}", b),
            Value::Doc(_) => String::from("<doc>"),
            Value::Unit => String::from("()"),
        },
    };

    // Prefer GUI when available and interactive, otherwise console
    if !is_noninteractive() {
        #[cfg(feature = "ui_egui")]
        {
            let _ = show_alert_gui(&msg);
        }
        #[cfg(not(feature = "ui_egui"))]
        {
            println!("[UI] {msg}");
        }
    } else {
        // Noninteractive: no GUI, no console prompt needed
    }
    Ok(prior)
}

#[cfg(feature = "ui_egui")]
fn show_alert_gui(message: &str) -> Result<()> {
    use eframe::{NativeOptions, egui};

    struct AlertApp {
        msg: String,
        acknowledged: bool,
    }
    impl eframe::App for AlertApp {
        fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.heading("TagSpeak Alert");
                ui.separator();
                ui.label(&self.msg);
                ui.add_space(8.0);
                if ui.button("OK").clicked() {
                    self.acknowledged = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
        }
    }

    let app = AlertApp {
        msg: message.to_string(),
        acknowledged: false,
    };
    let _ = eframe::run_native(
        "TagSpeak Alert",
        NativeOptions::default(),
        Box::new(|_cc| Box::new(app)),
    );
    Ok(())
}
