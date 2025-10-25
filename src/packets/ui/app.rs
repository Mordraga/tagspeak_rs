use anyhow::Result;
use crate::kernel::{Packet, Runtime, Value};
use crate::kernel::ast::{Arg, Node};
use crate::ui::tree as scene;
use std::sync::atomic::{AtomicUsize, Ordering};

// Local unique id counter for anonymous layout scopes
static LAYOUT_AUTO_COUNTER: AtomicUsize = AtomicUsize::new(0);

pub fn handle(rt: &mut Runtime, p: &Packet) -> Result<Value> {
    let title = match p.arg.as_ref() {
        Some(Arg::Str(s)) => s.clone(),
        Some(Arg::Ident(id)) => id.clone(),
        _ => "App".to_string(),
    };
    let body = p.body.as_ref().ok_or_else(|| anyhow::anyhow!("app requires a body"))?;

    let (mut children, mut layouts) = parse_body(rt, body)?;
    let mut root = scene::TagNode::new(scene::NodeKind::Window { title });
    root.children.append(&mut children);
    let mut root_layout = scene::LayoutIntent::default();

    // Build set of region ids for validation
    use std::collections::HashSet;
    fn collect_ids(node: &scene::TagNode, ids: &mut HashSet<String>) {
        for ch in &node.children {
            if let scene::NodeKind::Region { id, .. } = &ch.kind { ids.insert(id.clone()); }
            collect_ids(ch, ids);
        }
    }
    let mut ids = HashSet::new();
    collect_ids(&root, &mut ids);

    // Validate duplicates per parent
    fn validate_duplicates(node: &scene::TagNode) {
        use std::collections::HashSet;
        let mut seen: HashSet<&str> = HashSet::new();
        for ch in &node.children {
            if let scene::NodeKind::Region { id, .. } = &ch.kind {
                if !seen.insert(id.as_str()) {
                    eprintln!("[layout] warning: duplicate region id '{}' under same parent", id);
                }
            }
        }
        for ch in &node.children { validate_duplicates(ch); }
    }
    validate_duplicates(&root);

    // Apply layout modifiers
    for (targets, attrs) in layouts.drain(..) {
        for t in targets {
            if t.eq_ignore_ascii_case("app") || t.eq_ignore_ascii_case("window") {
                if attrs.direction.is_some() { root_layout.direction = attrs.direction.clone(); }
                if attrs.order.is_some() { root_layout.order = attrs.order; }
                if attrs.location.is_some() { root_layout.location = attrs.location.clone(); }
                if attrs.behavior.is_some() { root_layout.behavior = attrs.behavior.clone(); }
                if attrs.spacing.is_some() { root_layout.spacing = attrs.spacing; }
                if attrs.padding.is_some() { root_layout.padding = attrs.padding; }
                if let Some(a) = attrs.align.clone() { root_layout.align = Some(a); }
                if let Some(w) = attrs.width.clone() { root_layout.width = Some(w); }
                if attrs.border.is_some() { root_layout.border = attrs.border; }
                if attrs.border_color.is_some() { root_layout.border_color = attrs.border_color; }
            } else {
                if !ids.contains(&t) {
                    eprintln!("[layout] warning: unknown layout target '{}'", t);
                }
                apply_layout(&mut root.children, &t, &attrs);
            }
        }
    }
    root.layout = root_layout;
    let tree = scene::TagTree::new(root);

    #[cfg(feature = "ui_egui")]
    {
        crate::ui::adapters::egui::render(&tree, rt)?;
        return Ok(rt.last.clone());
    }
    #[cfg(not(feature = "ui_egui"))]
    {
        println!("[UI] app: <rendered by adapter>");
        return Ok(Value::Unit);
    }
}

fn parse_body(_rt: &Runtime, body: &Vec<Node>) -> Result<(Vec<scene::TagNode>, Vec<(Vec<String>, scene::LayoutIntent)>)> {
    let mut nodes: Vec<scene::TagNode> = Vec::new();
    let mut layouts: Vec<(Vec<String>, scene::LayoutIntent)> = Vec::new();
    for n in body {
        match n {
            Node::Packet(pkt) => {
                match (pkt.ns.as_deref(), pkt.op.as_str()) {
                    // [frame:id@"Label"]{ ... }
                    (Some("frame"), id) => {
                        let label = pkt.arg.as_ref().and_then(|a| match a { Arg::Str(s) => Some(s.clone()), Arg::Ident(s) => Some(s.clone()), _ => None });
                        let children = pkt.body.as_ref().map(|b| parse_children(_rt, b)).transpose()?.unwrap_or_default();
                        let mut node = scene::TagNode::new(scene::NodeKind::Region { id: id.to_string(), label });
                        node.children = children;
                        nodes.push(node);
                    }
                    // [layout(...)]{ ... } sugar: inline layout wrapper around children
                    // Also allow bare [layout]{ ... } for a neutral grouping scope
                    (None, op) if (op == "layout" || op.starts_with("layout(")) && pkt.body.is_some() => {
                        let attrs = if op.starts_with("layout(") { parse_layout_attrs(op) } else { scene::LayoutIntent::default() };
                        let children = pkt.body.as_ref().map(|b| parse_children(_rt, b)).transpose()?.unwrap_or_default();
                        let id = pkt.arg.as_ref().and_then(|a| match a { Arg::Str(s) => Some(s.clone()), Arg::Ident(s) => Some(s.clone()), _ => None })
                            .unwrap_or_else(|| format!("layout_scope_{}", LAYOUT_AUTO_COUNTER.fetch_add(1, Ordering::Relaxed)));
                        let mut node = scene::TagNode::new(scene::NodeKind::Region { id, label: None });
                        // Apply only in-frame attributes; ignore 'location' to keep scope inside parent
                        node.layout.direction = attrs.direction;
                        node.layout.order = attrs.order;
                        node.layout.behavior = attrs.behavior;
                        node.layout.spacing = attrs.spacing;
                        node.layout.padding = attrs.padding;
                        node.layout.align = attrs.align;
                        node.layout.width = attrs.width;
                        node.layout.border = attrs.border;
                        node.layout.border_color = attrs.border_color;
                        node.children = children;
                        nodes.push(node);
                    }
                    // [label@"text"]
                    (None, "label") => {
                        match pkt.arg.as_ref() {
                            Some(Arg::Str(s)) => nodes.push(scene::TagNode::new(scene::NodeKind::Text { text: s.clone() })),
                            Some(Arg::Ident(id)) => nodes.push(scene::TagNode::new(scene::NodeKind::TextVar { var: id.clone() })),
                            _ => nodes.push(scene::TagNode::new(scene::NodeKind::Text { text: String::new() })),
                        }
                    }
                    // [button@"label"]{ [call@fn] }
                    (None, "button") => {
                        let label = pkt.arg.as_ref().and_then(|a| match a { Arg::Str(s) => Some(s.clone()), Arg::Ident(s) => Some(s.clone()), _ => None }).unwrap_or_else(|| "Button".to_string());
                        let action = extract_button_action(pkt.body.as_ref());
                        nodes.push(scene::TagNode::new(scene::NodeKind::Button { label, action }));
                    }
                    // [textedit@var] or [textbox@var]
                    (None, "textedit") | (None, "textbox") => {
                        let var = pkt.arg.as_ref().and_then(|a| match a { Arg::Ident(s) => Some(s.clone()), Arg::Str(s) => Some(s.clone()), _ => None }).unwrap_or_else(|| "input".to_string());
                        nodes.push(scene::TagNode::new(scene::NodeKind::TextBox { var }));
                    }
                    // [checkbox:var@"Label"]
                    (Some("checkbox"), var) => {
                        let label = pkt.arg.as_ref().and_then(|a| match a { Arg::Str(s) => Some(s.clone()), Arg::Ident(s) => Some(s.clone()), _ => None });
                        let mut node = scene::TagNode::new(scene::NodeKind::Checkbox { var: var.to_string(), label });
                        nodes.push(node);
                    }
                    // [separator]
                    (None, "separator") => nodes.push(scene::TagNode::new(scene::NodeKind::Separator)),
                    // [spacer@px]
                    (None, "spacer") => {
                        let px = pkt.arg.as_ref().and_then(|a| match a { Arg::Number(n) => Some(*n as f32), Arg::Ident(s) => s.parse::<f32>().ok(), Arg::Str(s) => s.parse::<f32>().ok(), _ => None }).unwrap_or(8.0);
                        nodes.push(scene::TagNode::new(scene::NodeKind::Spacer { px }));
                    }
                    // [layout(params)@targets]
                    (None, op) if op.starts_with("layout(") => {
                        let attrs = parse_layout_attrs(op);
                        let targets = pkt.arg.as_ref().and_then(|a| match a { Arg::Str(s) => Some(s.clone()), Arg::Ident(s) => Some(s.clone()), _ => None }).unwrap_or_default();
                        let targets: Vec<String> = targets.split(',').map(|t| t.trim().to_string()).filter(|t| !t.is_empty()).collect();
                        layouts.push((targets, attrs));
                    }
                    // [popup] could be allowed under app; treat as a region with a special id for now or ignore
                    _ => { /* ignore for now */ }
                }
            }
            Node::Block(inner) | Node::Chain(inner) => {
                let (mut ch, mut ly) = parse_body(_rt, inner)?;
                nodes.append(&mut ch);
                layouts.append(&mut ly);
            }
            Node::If { .. } => {}
        }
    }
    Ok((nodes, layouts))
}

fn parse_children(rt: &Runtime, body: &Vec<Node>) -> Result<Vec<scene::TagNode>> {
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

fn parse_layout_attrs(op: &str) -> scene::LayoutIntent {
    let inner = op.trim().trim_start_matches("layout");
    let inner = inner.strip_prefix('(').and_then(|s| s.strip_suffix(')')).unwrap_or("");
    let mut out = scene::LayoutIntent::default();
    for part in inner.split(',') {
        let mut kv = part.splitn(2, '=');
        let k = kv.next().map(|s| s.trim()).unwrap_or("");
        let v = kv.next().map(|s| s.trim()).unwrap_or("");
        match k {
            "direction" => {
                out.direction = match v.trim_matches('"') {
                    "horizontal" => Some(scene::Direction::Horizontal),
                    "vertical" => Some(scene::Direction::Vertical),
                    _ => out.direction,
                };
            }
            "location" => {
                out.location = match v.trim_matches('"') {
                    "top" => Some(scene::Location::Top),
                    "bottom" => Some(scene::Location::Bottom),
                    "left" => Some(scene::Location::Left),
                    "right" => Some(scene::Location::Right),
                    "center" => Some(scene::Location::Center),
                    _ => out.location,
                };
            }
            "order" => { if let Ok(n) = v.parse::<u32>() { out.order = Some(n); } }
            "behavior" => {
                let vv = v.trim();
                if vv.starts_with("grid") {
                    if let Some(args) = vv.strip_prefix("grid").and_then(|s| s.strip_prefix('(')).and_then(|s| s.strip_suffix(')')) {
                        let mut it = args.split(',').map(|x| x.trim()).filter(|s| !s.is_empty());
                        let cols = it.next().and_then(|s| s.parse::<u32>().ok()).unwrap_or(1);
                        let rows = it.next().and_then(|s| s.parse::<u32>().ok()).unwrap_or(1);
                        out.behavior = Some(scene::LayoutBehavior::Grid { columns: cols, rows });
                    }
                } else {
                    out.behavior = Some(scene::LayoutBehavior::Flex);
                }
            }
            "spacing" => { if let Ok(n) = v.parse::<f32>() { out.spacing = Some(n); } }
            "padding" => { if let Ok(n) = v.parse::<f32>() { out.padding = Some(n); } }
            "align" => {
                out.align = match v.trim_matches('"') {
                    "start" => Some(scene::Align::Start),
                    "center" => Some(scene::Align::Center),
                    "end" => Some(scene::Align::End),
                    _ => out.align,
                };
            }
            "width" => {
                let vv = v.trim_matches('"');
                if vv.eq_ignore_ascii_case("fill") {
                    out.width = Some(scene::Width::Fill);
                } else if let Ok(px) = vv.parse::<f32>() {
                    out.width = Some(scene::Width::Px(px));
                }
            }
            "border" => {
                if let Ok(px) = v.parse::<f32>() { out.border = Some(px.max(0.0)); }
            }
            "border_color" => {
                let vv = v.trim_matches('"');
                if let Some((r,g,b,a)) = parse_hex_rgba(vv) { out.border_color = Some((r,g,b,a)); }
            }
            _ => {}
        }
    }
    out
}

fn parse_hex_rgba(s: &str) -> Option<(u8,u8,u8,u8)> {
    let hex = s.strip_prefix('#').unwrap_or(s);
    let hex = hex.trim();
    if hex.len() == 6 {
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        return Some((r,g,b,255));
    } else if hex.len() == 8 {
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        let a = u8::from_str_radix(&hex[6..8], 16).ok()?;
        return Some((r,g,b,a));
    }
    None
}

fn apply_layout(nodes: &mut [scene::TagNode], target: &str, attrs: &scene::LayoutIntent) {
    for n in nodes.iter_mut() {
        if let scene::NodeKind::Region { id, .. } = &n.kind {
            if id == target {
                if attrs.direction.is_some() { n.layout.direction = attrs.direction.clone(); }
                if attrs.order.is_some() { n.layout.order = attrs.order; }
                if attrs.location.is_some() { n.layout.location = attrs.location.clone(); }
                if attrs.behavior.is_some() { n.layout.behavior = attrs.behavior.clone(); }
                if attrs.spacing.is_some() { n.layout.spacing = attrs.spacing; }
                if attrs.padding.is_some() { n.layout.padding = attrs.padding; }
                if let Some(a) = attrs.align.clone() { n.layout.align = Some(a); }
                if let Some(w) = attrs.width.clone() { n.layout.width = Some(w); }
                if attrs.border.is_some() { n.layout.border = attrs.border; }
                if attrs.border_color.is_some() { n.layout.border_color = attrs.border_color; }
            }
        }
        apply_layout(&mut n.children, target, attrs);
    }
}
