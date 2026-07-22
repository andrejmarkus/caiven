pub struct SourceFile {
    pub consts: Vec<ConstDecl>,
    pub globals: Vec<LetDecl>,
    pub init_block: Option<Block>,
    pub loop_block: Option<Block>,
    pub functions: Vec<FnDecl>,
}

pub struct ConstDecl {
    pub name: String,
    pub value: u32,
    pub line: usize,
}

pub struct LetDecl {
    pub name: String,
    pub init: Expr,
    pub line: usize,
}

pub struct FnDecl {
    pub name: String,
    pub params: Vec<String>,
    pub is_variadic: bool,
    pub body: Block,
    pub line: usize,
}

pub type Block = Vec<Stmt>;

#[derive(Debug, Clone)]
pub enum Stmt {
    Local {
        names: Vec<String>,
        inits: Vec<Expr>,
        line: usize,
    },
    Assign {
        target: String,
        value: Expr,
        line: usize,
    },
    Do {
        body: Block,
        line: usize,
    },
    While {
        cond: Expr,
        body: Block,
        line: usize,
    },
    Repeat {
        body: Block,
        cond: Expr,
        line: usize,
    },
    If {
        cond: Expr,
        then_block: Block,
        elseif_clauses: Vec<(Expr, Block)>,
        else_block: Option<Block>,
        line: usize,
    },
    NumericFor {
        var: String,
        start: Expr,
        stop: Expr,
        step: Option<Expr>,
        body: Block,
        line: usize,
    },
    Return {
        values: Vec<Expr>,
        line: usize,
    },
    Break {
        line: usize,
    },
    ExprStmt {
        expr: Expr,
        line: usize,
    },
    SetField {
        table: Expr,
        name: String,
        value: Expr,
        line: usize,
    },
    SetIndex {
        table: Expr,
        key: Expr,
        value: Expr,
        line: usize,
    },
    GenericFor {
        key_var: String,
        val_var: String,
        table: Expr,
        body: Block,
        line: usize,
    },
}

#[derive(Debug, Clone)]
pub enum Expr {
    Nil(usize),
    True(usize),
    False(usize),
    Number(u32, usize),
    Str(String, usize),
    Var(String, usize),
    BinOp {
        op: BinOp,
        left: Box<Expr>,
        right: Box<Expr>,
        line: usize,
    },
    UnOp {
        op: UnOp,
        expr: Box<Expr>,
        line: usize,
    },
    Call {
        func: Box<Expr>,
        args: Vec<Expr>,
        line: usize,
    },
    // Phase 6d stubs — lower emits NotImplemented
    Table {
        fields: Vec<TableField>,
        line: usize,
    },
    Func {
        params: Vec<String>,
        body: Block,
        line: usize,
    },
    Index {
        table: Box<Expr>,
        key: Box<Expr>,
        line: usize,
    },
    Field {
        table: Box<Expr>,
        name: String,
        line: usize,
    },
    Varargs(usize),
}

impl Expr {
    pub fn line(&self) -> usize {
        match self {
            Expr::Nil(l) | Expr::True(l) | Expr::False(l) => *l,
            Expr::Number(_, l) | Expr::Str(_, l) | Expr::Var(_, l) => *l,
            Expr::BinOp { line, .. } => *line,
            Expr::UnOp { line, .. } => *line,
            Expr::Call { line, .. } => *line,
            Expr::Table { line, .. } => *line,
            Expr::Func { line, .. } => *line,
            Expr::Index { line, .. } => *line,
            Expr::Field { line, .. } => *line,
            Expr::Varargs(l) => *l,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    Concat,
    Eq,
    NotEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    And,
    Or,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnOp {
    Neg,
    Not,
    Len,
}

#[derive(Debug, Clone)]
pub enum TableField {
    IndexField { key: Expr, value: Expr },
    NameField { name: String, value: Expr },
    ValueField { value: Expr },
}
