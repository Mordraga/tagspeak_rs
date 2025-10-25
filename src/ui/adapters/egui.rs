use crate::kernel::Runtime;
use crate::ui::tree::*;
use anyhow::Result;
use eframe::{egui, NativeOptions};
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct ActionOut { call: Option<String>, scope: Option<String> }

pub fn render(tree: &TagTree, rt: &mut Runtime) -> Result<()> {
    struct App {
        root: TagNode,
        action: Arc<Mutex<ActionOut>>,
        rt_ptr: *mut Runtime,
    }
    impl eframe::App for App {
        fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
            // Partition by location and render with real panels
            let mut top_children: Vec<TagNode> = Vec::new();
            let mut bottom_children: Vec<TagNode> = Vec::new();
            let mut left_children: Vec<TagNode> = Vec::new();
            let mut right_children: Vec<TagNode> = Vec::new();
            let mut center_children: Vec<TagNode> = Vec::new();
            let mut others_children: Vec<TagNode> = Vec::new();

            for ch in &self.root.children {
                match ch.kind {
                    NodeKind::Region { .. } => match ch.layout.location {
                        Some(Location::Top) => top_children.push(ch.clone()),
                        Some(Location::Bottom) => bottom_children.push(ch.clone()),
                        Some(Location::Left) => left_children.push(ch.clone()),
                        Some(Location::Right) => right_children.push(ch.clone()),
                        _ => center_children.push(ch.clone()),
                    },
                    _ => others_children.push(ch.clone()),
                }
            }

            if !top_children.is_empty() {
                egui::TopBottomPanel::top("tgsk_top").show(ctx, |ui| {
                    let mut node = TagNode { kind: NodeKind::Region { id: "top".into(), label: None }, layout: LayoutIntent::default(), props: Default::default(), children: top_children.clone() };
                    render_region(ui, &mut node, &self.action, self.rt_ptr);
                });
            }
            if !left_children.is_empty() {
                egui::SidePanel::left("tgsk_left").show(ctx, |ui| {
                    let mut node = TagNode { kind: NodeKind::Region { id: "left".into(), label: None }, layout: LayoutIntent::default(), props: Default::default(), children: left_children.clone() };
                    render_region(ui, &mut node, &self.action, self.rt_ptr);
                });
            }
            if !right_children.is_empty() {
                egui::SidePanel::right("tgsk_right").show(ctx, |ui| {
                    let mut node = TagNode { kind: NodeKind::Region { id: "right".into(), label: None }, layout: LayoutIntent::default(), props: Default::default(), children: right_children.clone() };
                    render_region(ui, &mut node, &self.action, self.rt_ptr);
                });
            }
            egui::CentralPanel::default().show(ctx, |ui| {
                if !center_children.is_empty() {
                    let mut node = TagNode { kind: NodeKind::Region { id: "center".into(), label: None }, layout: LayoutIntent::default(), props: Default::default(), children: center_children.clone() };
                    render_region(ui, &mut node, &self.action, self.rt_ptr);
                }
                if !others_children.is_empty() {
                    let mut node = TagNode { kind: NodeKind::Region { id: "others".into(), label: None }, layout: LayoutIntent::default(), props: Default::default(), children: others_children.clone() };
                    render_region(ui, &mut node, &self.action, self.rt_ptr);
                }
            });
            if !bottom_children.is_empty() {
                egui::TopBottomPanel::bottom("tgsk_bottom").show(ctx, |ui| {
                    let mut node = TagNode { kind: NodeKind::Region { id: "bottom".into(), label: None }, layout: LayoutIntent::default(), props: Default::default(), children: bottom_children.clone() };
                    render_region(ui, &mut node, &self.action, self.rt_ptr);
                });
            }

            // Popups
            for ch in self.root.children.iter_mut() {
                if let NodeKind::Popup { title } = &ch.kind {
                    egui::Window::new(title.clone()).show(ctx, |ui| {
                        let mut single_root = TagNode { kind: NodeKind::Region { id: "popup_root".to_string(), label: None }, layout: LayoutIntent::default(), props: Default::default(), children: ch.children.clone() };
                        render_region(ui, &mut single_root, &self.action, self.rt_ptr);
                    });
                }
            }

            // Drain queued action (call + scope) outside the lock
            let (pending_call, pending_scope) = {
                let mut g = self.action.lock().unwrap();
                (g.call.take(), g.scope.take())
            };
            if let Some(call) = pending_call {
                unsafe {
                    let rt: &mut Runtime = &mut *self.rt_ptr;
                    // Capture scope for context-bound writes during this call
                    let prev_cap = rt.get_var("__scope_capture");
                    if let Some(sc) = pending_scope { let _ = rt.set_var("__scope_capture", crate::kernel::values::Value::Str(sc)); }
                    let packet = crate::kernel::ast::Packet { ns: None, op: "call".to_string(), arg: Some(crate::kernel::ast::Arg::Str(call)), body: None };
                    let _ = rt.eval(&crate::kernel::ast::Node::Packet(packet));
                    // restore
                    match prev_cap {
                        Some(v) => { let _ = rt.set_var("__scope_capture", v); }
                        None => { let _ = rt.set_var("__scope_capture", crate::kernel::values::Value::Unit); }
                    }
                }
            }
        }
    }

    fn render_region(ui: &mut egui::Ui, node: &mut TagNode, action: &Arc<Mutex<ActionOut>>, rt_ptr: *mut Runtime) {
        // Partition children by location for a basic region layout
        let mut top = Vec::new();
        let mut bottom = Vec::new();
        let mut left = Vec::new();
        let mut right = Vec::new();
        let mut center = Vec::new();
        let mut others = Vec::new();

        for ch in node.children.iter_mut() {
            match ch.kind {
                NodeKind::Region { .. } => match ch.layout.location {
                    Some(Location::Top) => top.push(ch),
                    Some(Location::Bottom) => bottom.push(ch),
                    Some(Location::Left) => left.push(ch),
                    Some(Location::Right) => right.push(ch),
                    _ => center.push(ch),
                },
                _ => others.push(ch),
            }
        }

        // helpers
        fn render_node(ui: &mut egui::Ui, node: &mut TagNode, action: &Arc<Mutex<ActionOut>>, rt_ptr: *mut Runtime) {
            match &mut node.kind {
                NodeKind::Region { id, label, .. } => {
                    // Apply spacing and padding scopes
                    let spacing = node.layout.spacing;
                    let padding = node.layout.padding;
                    let align = node.layout.align.clone();
                    let width = node.layout.width.clone();
                    let border = node.layout.border;
                    let border_color = node.layout.border_color;
                    let debug = std::env::var("TAGSPEAK_UI_DEBUG")
                        .map(|v| matches!(v.as_str(), "1" | "true" | "y" | "yes"))
                        .unwrap_or(false);

                    // Set __ui_scope to region id during render
                    let prev_scope = unsafe { (&*rt_ptr).get_var("__ui_scope") };
                    unsafe { let _ = (&mut *rt_ptr).set_var("__ui_scope", crate::kernel::values::Value::Str(id.clone())); }

                    // Apply width intent (default to fill for center-like regions)
                    if let Some(w) = width {
                        match w {
                            Width::Fill => { ui.set_min_width(ui.available_width()); },
                            Width::Px(px) => { ui.set_min_width(px); ui.set_max_width(px); },
                        }
                    } else {
                        // Default-fill only when explicitly marked Center; avoid monopolizing width in inline/horizontal scopes
                        if matches!(node.layout.location, Some(Location::Center)) {
                            ui.set_min_width(ui.available_width());
                        }
                    }

                    // Apply spacing and padding by scoping style and wrapping frame
                    let mut render_body = |ui: &mut egui::Ui| {
                        if let Some(lbl) = label { ui.heading(lbl.as_str()); }
                        match &node.layout.behavior {
                            Some(LayoutBehavior::Grid { columns, .. }) => {
                                let grid = egui::Grid::new(format!("grid_{:p}", &node.children as *const _)).num_columns(*columns as usize);
                                grid.show(ui, |ui| {
                                    for (i, ch) in node.children.iter_mut().enumerate() {
                                        render_node(ui, ch, action, rt_ptr);
                                        if (i + 1) % (*columns as usize) == 0 { ui.end_row(); }
                                    }
                                });
                            }
                            _ => {
                                let egui_align = match align { Some(Align::Start) | None => egui::Align::Min, Some(Align::Center) => egui::Align::Center, Some(Align::End) => egui::Align::Max };
                                match node.layout.direction {
                                    Some(Direction::Horizontal) => {
                                        // Default: evenly split horizontal space across children unless any child has explicit pixel width.
                                        let has_px = node.children.iter().any(|c| matches!(c.layout.width, Some(Width::Px(_))));
                                        let count = node.children.len();
                                        if count > 1 && !has_px {
                                            ui.columns(count, |cols| {
                                                for (i, ch) in node.children.iter_mut().enumerate() {
                                                    render_node(&mut cols[i], ch, action, rt_ptr);
                                                }
                                            });
                                        } else {
                                            let _ = ui.with_layout(egui::Layout::left_to_right(egui_align), |ui| {
                                                for ch in node.children.iter_mut() { render_node(ui, ch, action, rt_ptr); }
                                            });
                                        }
                                    }
                                    _ => { let _ = ui.with_layout(egui::Layout::top_down(egui_align), |ui| {
                                        for ch in node.children.iter_mut() { render_node(ui, ch, action, rt_ptr); }
                                    }); },
                                };
                            }
                        }
                    };

                    let want_border = debug || border.unwrap_or(0.0) > 0.0;
                    let stroke_width = border.unwrap_or(1.0).max(0.0);
                    let color = border_color.map(|(r,g,b,a)| egui::Color32::from_rgba_unmultiplied(r, g, b, a))
                        .unwrap_or(egui::Color32::from_rgb(80, 160, 255));

                    if let Some(p) = padding { 
                        let mut frame = egui::Frame::none().inner_margin(egui::Margin::same(p));
                        if want_border { frame = frame.stroke(egui::Stroke::new(stroke_width, color)); }
                        frame.show(ui, |ui| {
                            if debug { ui.small(format!("[region:{}]", id)); }
                            if let Some(sp) = spacing {
                                ui.scope(|ui| {
                                    let mut st: egui::Style = ui.style().as_ref().clone();
                                    st.spacing.item_spacing = egui::vec2(sp, sp);
                                    ui.set_style(st);
                                    render_body(ui);
                                });
                            } else { render_body(ui); }
                        });
                    } else if let Some(sp) = spacing {
                        ui.scope(|ui| {
                            let mut st: egui::Style = ui.style().as_ref().clone();
                            st.spacing.item_spacing = egui::vec2(sp, sp);
                            ui.set_style(st);
                            if want_border {
                                egui::Frame::none().stroke(egui::Stroke::new(stroke_width, color)).show(ui, |ui| {
                                    if debug { ui.small(format!("[region:{}]", id)); }
                                    render_body(ui);
                                });
                            } else {
                                if debug { ui.small(format!("[region:{}]", id)); }
                                render_body(ui);
                            }
                        });
                    } else {
                        if want_border {
                            egui::Frame::none()
                                .stroke(egui::Stroke::new(stroke_width, color))
                                .show(ui, |ui| {
                                    if debug { ui.small(format!("[region:{}]", id)); }
                                    render_body(ui);
                                });
                        } else {
                            render_body(ui);
                        }
                    }

                    // Restore previous scope
                    unsafe {
                        let rt: &mut Runtime = &mut *rt_ptr;
                        match prev_scope {
                            Some(v) => { let _ = rt.set_var("__ui_scope", v); }
                            None => { let _ = rt.set_var("__ui_scope", crate::kernel::values::Value::Unit); }
                        }
                    }
                }
                NodeKind::Text { text } => { ui.label(text.as_str()); }
                NodeKind::TextVar { var } => {
                    let text = unsafe {
                        let rt: &mut Runtime = &mut *rt_ptr;
                        match rt.get_var(var) {
                            Some(crate::kernel::values::Value::Str(s)) => s,
                            Some(crate::kernel::values::Value::Num(n)) => format!("{}", n),
                            Some(crate::kernel::values::Value::Bool(b)) => format!("{}", b),
                            Some(crate::kernel::values::Value::Doc(_)) => String::from("<doc>"),
                            _ => String::new(),
                        }
                    };
                    ui.label(text);
                }
                NodeKind::Button { label, action: act } => {
                    if ui.button(label.as_str()).clicked() {
                        if let Some(a) = act.clone() {
                            // capture current scope id for context-bound writes
                            let current_scope = unsafe { (&*rt_ptr).get_var("__ui_scope") };
                            {
                                let mut guard = action.lock().unwrap();
                                guard.call = Some(a);
                                if let Some(crate::kernel::values::Value::Str(s)) = current_scope { guard.scope = Some(s); }
                            }
                            // Ensure a follow-up frame runs to process the action
                            ui.ctx().request_repaint();
                        }
                    }
                }
                NodeKind::TextBox { var } => {
                    let mut buf = String::new();
                    unsafe {
                        let rt: &mut Runtime = &mut *rt_ptr;
                        if let Some(crate::kernel::values::Value::Str(s)) = rt.get_var(var) { buf = s; }
                    }
                    let resp = ui.text_edit_singleline(&mut buf);
                    if resp.changed() {
                        unsafe {
                            let rt: &mut Runtime = &mut *rt_ptr;
                            let _ = rt.set_var(var, crate::kernel::values::Value::Str(buf));
                        }
                    }
                }
                NodeKind::Checkbox { var, label } => {
                    let mut checked = false;
                    unsafe {
                        let rt: &mut Runtime = &mut *rt_ptr;
                        if let Some(crate::kernel::values::Value::Bool(b)) = rt.get_var(var) { checked = b; }
                    }
                    let resp = if let Some(lbl) = label { ui.checkbox(&mut checked, lbl.as_str()) } else { ui.checkbox(&mut checked, "") };
                    if resp.changed() {
                        unsafe {
                            let rt: &mut Runtime = &mut *rt_ptr;
                            let _ = rt.set_var(var, crate::kernel::values::Value::Bool(checked));
                        }
                    }
                }
                NodeKind::Separator => { ui.separator(); }
                NodeKind::Spacer { px } => { ui.add_space(*px); }
                NodeKind::Window { .. } => {/* handled at root */}
                NodeKind::Popup { .. } => {/* handled at root */}
            }
        }

        // Optional sort by order while keeping declaration order for ties
        let mut sort_by_order = |v: &mut Vec<&mut TagNode>| {
            if v.iter().any(|n| n.layout.order.is_some()) {
                v.sort_by_key(|n| n.layout.order.unwrap_or(u32::MAX));
            }
        };
        sort_by_order(&mut top);
        sort_by_order(&mut left);
        sort_by_order(&mut right);
        sort_by_order(&mut center);
        sort_by_order(&mut bottom);

        // Top region (horizontal by default within its frame)
        if !top.is_empty() {
            ui.horizontal(|ui| { for n in &mut top { render_node(ui, *n, action, rt_ptr); } });
            ui.separator();
        }
        // middle row: left | center | right
        ui.horizontal(|ui| {
            if !left.is_empty() {
                ui.vertical(|ui| { for n in &mut left { render_node(ui, *n, action, rt_ptr); } });
            }
            // Ensure center region claims available width when no explicit width is set
            ui.vertical(|ui| {
                // Claim remaining width for the center column container
                ui.set_min_width(ui.available_width());
                for n in &mut center {
                    if n.layout.width.is_none() {
                        n.layout.width = Some(Width::Fill);
                    }
                    render_node(ui, *n, action, rt_ptr);
                }
            });
            if !right.is_empty() {
                ui.vertical(|ui| { for n in &mut right { render_node(ui, *n, action, rt_ptr); } });
            }
        });
        if !bottom.is_empty() {
            ui.separator();
            ui.horizontal(|ui| { for n in &mut bottom { render_node(ui, *n, action, rt_ptr); } });
        }
        for n in others { render_node(ui, n, action, rt_ptr); }
    }

    let action = Arc::new(Mutex::new(ActionOut::default()));
    let app = App { root: tree.root.clone(), action: action.clone(), rt_ptr: rt as *mut Runtime };
    let _ = eframe::run_native("TagSpeak UI", NativeOptions::default(), Box::new(|_cc| Box::new(app)));
    Ok(())
}
