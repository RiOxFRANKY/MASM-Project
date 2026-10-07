use super::expr::Expr;
use super::types::TypeKind;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Model {
    Tiny,
    Small,
    Medium,
    Compact,
    Large,
}

impl Model {
    pub fn far_code(self) -> bool {
        matches!(self, Model::Medium | Model::Large)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SimpleSegment {
    Code,
    Data,
    Bss,
    Const,
    Stack,
}

#[derive(Clone, Debug)]
pub enum Stmt {
    Label(String),
    Instruction { prefixes: Vec<u8>, mnemonic: String, operands: Vec<Expr> },
    Data { name: Option<String>, kind: TypeKind, items: Vec<Expr> },
    Equ { name: String, value: Expr },
    Assign { name: String, value: Expr },
    Segment { name: String, align: u32, stack: bool, class: String },
    Ends(String),
    Group { name: String, members: Vec<String> },
    Proc { name: String, distance: Option<TypeKind> },
    Endp(String),
    LabelDirective { name: String, kind: TypeKind },
    Model(Model),
    Stack(Option<Expr>),
    Simple(SimpleSegment),
    Org(Expr),
    Align(u32),
    End(Option<Expr>),
}

#[derive(Clone, Debug)]
pub struct Line {
    pub number: usize,
    pub stmt: Stmt,
}
