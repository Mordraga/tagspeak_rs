// src/kernel/ast.rs
#[derive(Debug, Clone)]
pub enum Node {
    Chain(Vec<Node>),
    Block(Vec<Node>),
    Packet(Packet),
    If {
        cond: BExpr,
        then_b: Vec<Node>,
        else_b: Vec<Node>,
    },
}

#[derive(Debug, Clone)]
pub struct Packet {
    pub ns: Option<String>,
    pub op: String,
    pub arg: Option<Arg>,
    pub body: Option<Vec<Node>>,
}

#[derive(Debug, Clone)]
pub enum Arg {
    Str(String),
    Ident(String),
    Number(f64),
    CondSrc(String), // for [if@( ... )]
}

#[derive(Debug, Clone)]
pub enum BExpr {
    Cmp {
        lhs: Box<Node>,
        cmp: Comparator,
        rhs: Box<Node>,
    },
    And(Box<BExpr>, Box<BExpr>),
    Or(Box<BExpr>, Box<BExpr>),
    Not(Box<BExpr>),
    Lit(String), // stores raw packet chain for runtime eval
}

#[derive(Debug, Clone)]
pub enum CmpBase {
    Eq,
    Lt,
    Gt,
}

#[derive(Debug, Clone)]
pub struct Comparator {
    pub base: CmpBase,
    pub include_eq: bool,
    pub negate: bool,
}
