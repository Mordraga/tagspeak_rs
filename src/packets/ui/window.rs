use crate::kernel::ast::{Arg, Node};
use crate::kernel::{Packet, Runtime, Value};
use anyhow::Result;

#[derive(Clone, Debug, Default)]
struct LayoutAttrs {
    direction: Option<Direction>,
    order: Option<u32>,
    location: Option<Location>,
    behavior: Option<LayoutBehavior>,
}

#[derive(Clone, Debug)]
enum Direction {
    Vertical,
    Horizontal,
}

#[derive(Clone, Debug)]
enum Location {
    Top,
    Bottom,
    Left,
    Right,
    Center,
}

#[derive(Clone, Debug)]
enum LayoutBehavior {
    Flex,
    Grid { columns: u32, rows: u32 },
}

#[derive(Clone, Debug)]
enum UiNode {
    Frame {
        id: String,
        label: Option<String>,
        layout: LayoutAttrs,
        children: Vec<UiNode>,
    },
    Label {
        text: String,
    },
    Button {
        label: String,
        action: Option<String>,
    },
    TextEdit {
        var: String,
    },
    Popup {
        title: String,
        children: Vec<UiNode>,
    },
}

pub fn handle(rt: &mut Runtime, p: &Packet) -> Result<Value> {
    let title = match p.arg.as_ref() {
        Some(Arg::Str(s)) => s.clone(),
        Some(Arg::Ident(id)) => id.clone(),
        _ => "Window".to_string(),
    };
    let body = p
        .body
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("ui:window requires a body"))?;

    let (mut nodes, mut layouts) = parse_body(rt, body)?;
    let mut root_layout = LayoutAttrs::default();
    // apply simple layout directives (direction, order) to frames by id
    for (targets, attrs) in layouts.drain(..) {
        for t in targets {
            if t.eq_ignore_ascii_case("window") {
                if attrs.direction.is_some() {
                    root_layout.direction = attrs.direction.clone();
                }
                if attrs.order.is_some() {
                    root_layout.order = attrs.order;
                }
                if attrs.location.is_some() {
                    root_layout.location = attrs.location.clone();
                }
                if attrs.behavior.is_some() {
                    root_layout.behavior = attrs.behavior.clone();
                }
            } else {
                apply_layout(&mut nodes, &t, &attrs);
            }
        }
    }

    // simple ordering by order attr if present
    sort_by_order(&mut nodes);

    // Render
    #[cfg(feature = "ui_egui")]
    {
        render_gui(rt, &title, &mut nodes, &root_layout)?;
        // For now, return last runtime value; button actions may have mutated it
        return Ok(rt.last.clone());
    }
    #[cfg(not(feature = "ui_egui"))]
    {
        // Console fallback: print a simple tree, no interactivity
        println!("[UI] window: {title}");
        print_tree(&nodes, 1);
        return Ok(Value::Unit);
    }
}

fn parse_body(
    _rt: &Runtime,
    body: &Vec<Node>,
) -> Result<(Vec<UiNode>, Vec<(Vec<String>, LayoutAttrs)>)> {
    let mut nodes = Vec::new();
    let mut layouts = Vec::new();
    for n in body {
        match n {
            Node::Packet(pkt) => {
                match (pkt.ns.as_deref(), pkt.op.as_str()) {
                    // [frame:id@"Label"]{ ... }
                    (Some("frame"), id) => {
                        let label = pkt.arg.as_ref().and_then(|a| match a {
                            Arg::Str(s) => Some(s.clone()),
                            Arg::Ident(s) => Some(s.clone()),
                            _ => None,
                        });
                        let children = pkt
                            .body
                            .as_ref()
                            .map(|b| parse_children(_rt, b))
                            .transpose()?
                            .unwrap_or_default();
                        nodes.push(UiNode::Frame {
                            id: id.to_string(),
                            label,
                            layout: LayoutAttrs::default(),
                            children,
                        });
                    }
                    // [label@"text"]
                    (None, "label") => {
                        let text = pkt
                            .arg
                            .as_ref()
                            .and_then(|a| match a {
                                Arg::Str(s) => Some(s.clone()),
                                Arg::Ident(s) => Some(s.clone()),
                                _ => None,
                            })
                            .unwrap_or_default();
                        nodes.push(UiNode::Label { text });
                    }
                    // [separator]
                    (None, "separator") => {
                        nodes.push(UiNode::Label {
                            text: "---".to_string(),
                        }); // placeholder for console tree
                    }
                    // [spacer@px]
                    (None, "spacer") => {
                        // store as a no-op in console tree
                        nodes.push(UiNode::Label {
                            text: String::from(""),
                        });
                    }
                    // [button@"label"]{ [call@fn] }
                    (None, "button") => {
                        let label = pkt
                            .arg
                            .as_ref()
                            .and_then(|a| match a {
                                Arg::Str(s) => Some(s.clone()),
                                Arg::Ident(s) => Some(s.clone()),
                                _ => None,
                            })
                            .unwrap_or_else(|| "Button".to_string());
                        let action = extract_button_action(pkt.body.as_ref());
                        nodes.push(UiNode::Button { label, action });
                    }
                    // [textedit@var] or [textbox@var]
                    (None, "textedit") | (None, "textbox") => {
                        let var = pkt
                            .arg
                            .as_ref()
                            .and_then(|a| match a {
                                Arg::Ident(s) => Some(s.clone()),
                                Arg::Str(s) => Some(s.clone()),
                                _ => None,
                            })
                            .unwrap_or_else(|| "input".to_string());
                        nodes.push(UiNode::TextEdit { var });
                    }
                    // [checkbox:var@"Label"]
                    (Some("checkbox"), var) => {
                        let label = pkt.arg.as_ref().and_then(|a| match a {
                            Arg::Str(s) => Some(s.clone()),
                            Arg::Ident(s) => Some(s.clone()),
                            _ => None,
                        });
                        // Represent as a label in console fallback tree; adapter will render actual checkbox
                        nodes.push(UiNode::Label {
                            text: format!("[ ] {}", label.unwrap_or_else(|| var.to_string())),
                        });
                    }
                    // [popup@"Title"]{ ... }
                    (None, "popup") => {
                        let title = pkt
                            .arg
                            .as_ref()
                            .and_then(|a| match a {
                                Arg::Str(s) => Some(s.clone()),
                                Arg::Ident(s) => Some(s.clone()),
                                _ => None,
                            })
                            .unwrap_or_else(|| "Popup".to_string());
                        let children = pkt
                            .body
                            .as_ref()
                            .map(|b| parse_children(_rt, b))
                            .transpose()?
                            .unwrap_or_default();
                        nodes.push(UiNode::Popup { title, children });
                    }
                    // [layout(params)@targets]
                    (None, op) if op.starts_with("layout(") => {
                        let attrs = parse_layout_attrs(op);
                        let targets = pkt
                            .arg
                            .as_ref()
                            .and_then(|a| match a {
                                Arg::Str(s) => Some(s.clone()),
                                Arg::Ident(s) => Some(s.clone()),
                                _ => None,
                            })
                            .unwrap_or_default();
                        let targets: Vec<String> = targets
                            .split(',')
                            .map(|t| t.trim().to_string())
                            .filter(|t| !t.is_empty())
                            .collect();
                        layouts.push((targets, attrs));
                    }
                    _ => {
                        // ignore other packets in UI body for now
                    }
                }
            }
            Node::Block(inner) | Node::Chain(inner) => {
                // flatten
                let (mut ch, mut ly) = parse_body(_rt, inner)?;
                nodes.append(&mut ch);
                layouts.append(&mut ly);
            }
            Node::If { .. } => { /* future: conditions inside UI */ }
        }
    }
    Ok((nodes, layouts))
}

fn parse_children(rt: &Runtime, body: &Vec<Node>) -> Result<Vec<UiNode>> {
    let (nodes, _layouts) = parse_body(rt, body)?;
    Ok(nodes)
}

fn extract_button_action(body: Option<&Vec<Node>>) -> Option<String> {
    let body = body?;
    for n in body {
        if let Node::Packet(p) = n {
            if p.ns.is_none() && p.op == "call" {
                if let Some(Arg::Ident(id)) | Some(Arg::Str(id)) = p.arg.as_ref() {
                    return Some(id.clone());
                }
            }
        }
    }
    None
}

fn parse_layout_attrs(op: &str) -> LayoutAttrs {
    // very light parser: look for direction=horizontal|vertical and order=number
    let inner = op.trim().trim_start_matches("layout");
    let inner = inner
        .strip_prefix('(')
        .and_then(|s| s.strip_suffix(')'))
        .unwrap_or("");
    let mut out = LayoutAttrs::default();
    for part in inner.split(',') {
        let mut kv = part.splitn(2, '=');
        let k = kv.next().map(|s| s.trim()).unwrap_or("");
        let v = kv.next().map(|s| s.trim()).unwrap_or("");
        match k {
            "direction" => {
                out.direction = match v.trim_matches('"') {
                    "horizontal" => Some(Direction::Horizontal),
                    "vertical" => Some(Direction::Vertical),
                    _ => out.direction,
                };
            }
            "location" => {
                out.location = match v.trim_matches('"') {
                    "top" => Some(Location::Top),
                    "bottom" => Some(Location::Bottom),
                    "left" => Some(Location::Left),
                    "right" => Some(Location::Right),
                    "center" => Some(Location::Center),
                    _ => out.location,
                };
            }
            "order" => {
                if let Ok(n) = v.parse::<u32>() {
                    out.order = Some(n);
                }
            }
            "behavior" => {
                let vv = v.trim();
                if vv.starts_with("grid") {
                    // behavior = grid(cols,rows)
                    if let Some(args) = vv
                        .strip_prefix("grid")
                        .and_then(|s| s.strip_prefix('('))
                        .and_then(|s| s.strip_suffix(')'))
                    {
                        let mut it = args.split(',').map(|x| x.trim()).filter(|s| !s.is_empty());
                        let cols = it.next().and_then(|s| s.parse::<u32>().ok()).unwrap_or(1);
                        let rows = it.next().and_then(|s| s.parse::<u32>().ok()).unwrap_or(1);
                        out.behavior = Some(LayoutBehavior::Grid {
                            columns: cols,
                            rows,
                        });
                    }
                } else {
                    out.behavior = Some(LayoutBehavior::Flex);
                }
            }
            _ => {}
        }
    }
    out
}

fn apply_layout(nodes: &mut [UiNode], target: &str, attrs: &LayoutAttrs) {
    for n in nodes.iter_mut() {
        match n {
            UiNode::Frame {
                id,
                layout,
                children,
                ..
            } => {
                if id == target {
                    if attrs.direction.is_some() {
                        layout.direction = attrs.direction.clone();
                    }
                    if attrs.order.is_some() {
                        layout.order = attrs.order;
                    }
                }
                apply_layout(children, target, attrs);
            }
            _ => {}
        }
    }
}

fn sort_by_order(nodes: &mut Vec<UiNode>) {
    nodes.sort_by(|a, b| order_of(a).cmp(&order_of(b)));
}
fn order_of(n: &UiNode) -> u32 {
    match n {
        UiNode::Frame { layout, .. } => layout.order.unwrap_or(u32::MAX / 2),
        _ => u32::MAX / 2,
    }
}

#[cfg(feature = "ui_egui")]
fn render_gui(
    rt: &mut Runtime,
    title: &str,
    nodes: &mut [UiNode],
    _root: &LayoutAttrs,
) -> Result<()> {
    use eframe::{NativeOptions, egui};
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct ActionOut {
        call: Option<String>,
    }

    struct WindowApp<'a> {
        nodes: Vec<UiNode>,
        action: Arc<Mutex<ActionOut>>,
        rt_ptr: *mut Runtime,
        _phantom: std::marker::PhantomData<&'a mut Runtime>,
        title: String,
    }

    impl<'a> eframe::App for WindowApp<'a> {
        fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
            // Render root scene into CentralPanel (full app canvas)
            egui::CentralPanel::default().show(ctx, |ui| {
                render_nodes(ui, &mut self.nodes, &self.action, self.rt_ptr);
            });

            // Render popup overlays (floating windows)
            for n in &mut self.nodes {
                if let UiNode::Popup { title, children } = n {
                    egui::Window::new(title.clone()).show(ctx, |ui| {
                        render_nodes(ui, children, &self.action, self.rt_ptr);
                    });
                }
            }

            // if an action has been set, execute it and close
            if let Some(call) = self.action.lock().unwrap().call.take() {
                unsafe {
                    // SAFETY: we ensure the runtime outlives the app run
                    let rt: &mut Runtime = &mut *self.rt_ptr;
                    let node = Node::Packet(Packet {
                        ns: None,
                        op: "call".to_string(),
                        arg: Some(Arg::Str(call)),
                        body: None,
                    });
                    let _ = rt.eval(&node);
                }
                // Keep window open; allow multiple interactions
            }
        }
    }

    fn render_nodes(
        ui: &mut egui::Ui,
        nodes: &mut [UiNode],
        action: &Arc<Mutex<ActionOut>>,
        rt_ptr: *mut Runtime,
    ) {
        // Partition frames by location to emulate top/bottom/left/right/center inside the window
        let mut top = Vec::new();
        let mut bottom = Vec::new();
        let mut left = Vec::new();
        let mut right = Vec::new();
        let mut center = Vec::new();
        let mut others = Vec::new();

        for n in nodes.iter_mut() {
            match n {
                UiNode::Frame { layout, .. } => match layout.location {
                    Some(Location::Top) => top.push(n),
                    Some(Location::Bottom) => bottom.push(n),
                    Some(Location::Left) => left.push(n),
                    Some(Location::Right) => right.push(n),
                    Some(Location::Center) | None => center.push(n),
                },
                _ => others.push(n),
            }
        }

        let mut render_frame = |ui: &mut egui::Ui, node: &mut UiNode| {
            if let UiNode::Frame {
                label,
                children,
                layout,
                ..
            } = node
            {
                if let Some(lbl) = label {
                    ui.heading(lbl.as_str());
                }
                match layout.behavior {
                    Some(LayoutBehavior::Grid { columns, .. }) => {
                        let grid = egui::Grid::new(format!("grid_{:p}", &*children as *const _))
                            .num_columns(columns as usize);
                        grid.show(ui, |ui| {
                            for (i, ch) in children.iter_mut().enumerate() {
                                render_leaf(ui, ch, action, rt_ptr);
                                if (i + 1) % (columns as usize) == 0 {
                                    ui.end_row();
                                }
                            }
                        });
                    }
                    _ => match layout.direction {
                        Some(Direction::Horizontal) => {
                            ui.horizontal(|ui| {
                                for ch in children.iter_mut() {
                                    render_leaf(ui, ch, action, rt_ptr);
                                }
                            });
                        }
                        _ => {
                            for ch in children.iter_mut() {
                                render_leaf(ui, ch, action, rt_ptr);
                            }
                        }
                    },
                }
            } else {
                render_leaf(ui, node, action, rt_ptr);
            }
        };

        // Top region
        for n in &mut top {
            render_frame(ui, *n);
        }
        if !top.is_empty() {
            ui.separator();
        }

        // Middle region with left | center | right
        ui.horizontal(|ui| {
            if !left.is_empty() {
                ui.vertical(|ui| {
                    for n in &mut left {
                        render_frame(ui, *n);
                    }
                });
            }
            ui.vertical(|ui| {
                for n in &mut center {
                    render_frame(ui, *n);
                }
            });
            if !right.is_empty() {
                ui.vertical(|ui| {
                    for n in &mut right {
                        render_frame(ui, *n);
                    }
                });
            }
        });

        if !bottom.is_empty() {
            ui.separator();
        }
        for n in &mut bottom {
            render_frame(ui, *n);
        }

        // Any non-frame nodes render at the end
        for n in others {
            render_leaf(ui, n, action, rt_ptr);
        }
    }

    fn render_leaf(
        ui: &mut egui::Ui,
        node: &mut UiNode,
        action: &Arc<Mutex<ActionOut>>,
        rt_ptr: *mut Runtime,
    ) {
        match node {
            UiNode::Label { text } => {
                ui.label(text.as_str());
            }
            UiNode::Button { label, action: act } => {
                if ui.button(label.as_str()).clicked() {
                    if let Some(a) = act.clone() {
                        action.lock().unwrap().call = Some(a);
                    }
                }
            }
            UiNode::TextEdit { var } => {
                // Pull current value from runtime, edit it, and write back
                let mut buf = String::new();
                unsafe {
                    let rt: &mut Runtime = &mut *rt_ptr;
                    if let Some(Value::Str(s)) = rt.get_var(var) {
                        buf = s;
                    }
                }
                let resp = ui.text_edit_singleline(&mut buf);
                if resp.changed() {
                    unsafe {
                        let rt: &mut Runtime = &mut *rt_ptr;
                        let _ = rt.set_var(var, Value::Str(buf));
                    }
                }
            }
            UiNode::Frame { .. } => { /* handled by render_frame */ }
            UiNode::Popup { .. } => { /* handled at top-level */ }
        }
    }

    let action = Arc::new(Mutex::new(ActionOut::default()));
    let app = WindowApp {
        nodes: nodes.to_vec(),
        action: action.clone(),
        rt_ptr: rt as *mut Runtime,
        _phantom: Default::default(),
        title: title.to_string(),
    };
    let _ = eframe::run_native(
        "TagSpeak UI",
        NativeOptions::default(),
        Box::new(|_cc| Box::new(app)),
    );
    Ok(())
}

#[cfg(not(feature = "ui_egui"))]
fn print_tree(nodes: &[UiNode], depth: usize) {
    let pad = "  ".repeat(depth);
    for n in nodes {
        match n {
            UiNode::Frame {
                id,
                label,
                children,
                ..
            } => {
                println!("{pad}frame {id} - {}", label.as_deref().unwrap_or(""));
                print_tree(children, depth + 1);
            }
            UiNode::Label { text } => println!("{pad}label: {text}"),
            UiNode::Button { label, .. } => println!("{pad}button: {label}"),
            UiNode::TextEdit { var } => println!("{pad}textedit: {var}"),
            UiNode::Popup { title, children } => {
                println!("{pad}popup: {title}");
                print_tree(children, depth + 1);
            }
        }
    }
}
