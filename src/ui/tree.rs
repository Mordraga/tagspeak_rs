use std::collections::HashMap;

#[derive(Clone, Debug)]
pub enum Direction {
    Vertical,
    Horizontal,
}

#[derive(Clone, Debug)]
pub enum Location {
    Top,
    Bottom,
    Left,
    Right,
    Center,
}

#[derive(Clone, Debug)]
pub enum LayoutBehavior {
    Flex,
    Grid { columns: u32, rows: u32 },
}

#[derive(Clone, Debug)]
pub enum Align {
    Start,
    Center,
    End,
}

#[derive(Clone, Debug)]
pub enum Width {
    Fill,
    Px(f32),
}

#[derive(Clone, Debug, Default)]
pub struct LayoutIntent {
    pub direction: Option<Direction>,
    pub order: Option<u32>,
    pub location: Option<Location>,
    pub behavior: Option<LayoutBehavior>,
    pub spacing: Option<f32>,
    pub padding: Option<f32>,
    pub align: Option<Align>,
    pub width: Option<Width>,
    pub border: Option<f32>,
    pub border_color: Option<(u8, u8, u8, u8)>,
}

#[derive(Clone, Debug)]
pub enum NodeKind {
    Window { title: String },
    Popup  { title: String },
    Region { id: String, label: Option<String> },
    Text { text: String },
    TextVar { var: String },
    Button { label: String, action: Option<String> },
    TextBox { var: String },
    Checkbox { var: String, label: Option<String> },
    Separator,
    Spacer { px: f32 },
}

#[derive(Clone, Debug)]
pub struct TagNode {
    pub kind: NodeKind,
    pub layout: LayoutIntent,
    pub props: HashMap<String, String>,
    pub children: Vec<TagNode>,
}

impl TagNode {
    pub fn new(kind: NodeKind) -> Self {
        Self { kind, layout: LayoutIntent::default(), props: HashMap::new(), children: Vec::new() }
    }
}

#[derive(Clone, Debug)]
pub struct TagTree {
    pub root: TagNode,
}

impl TagTree {
    pub fn new(root: TagNode) -> Self { Self { root } }
}
