use std::collections::BTreeMap;

pub mod expression;
pub mod fn_definition;
pub mod pattern;
pub mod program;

#[derive(Clone, Debug)]
pub enum Expression {
  Variable {
    name: String,
  },
  Number {
    value: i32,
  },
  Atom {
    value: String,
  },
  String {
    value: String,
  },
  Let {
    bind: String,
    value: Expr,
    next: Expr,
  },
  Match {
    tree: Tree,
    actions: Vec<Expression>,
  },
  Tuple {
    elements: Vec<Expression>,
  },
  Binary {
    op: Operation,
    lhs: Expr,
    rhs: Expr,
  },
  Call {
    callee: Expr,
    arguments: Vec<Expression>,
  },
  Access {
    expr: Expr,
    idx: usize,
  },
}

#[derive(Clone, Debug)]
pub enum Tree {
  Failure,
  Leaf(usize),
  Switch(Box<Occurrence>, Vec<(Case, Tree)>, Box<Tree>),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Case {
  // Variable(String),
  Number(i32),
  String(String),
  Atom(String),
  Tuple(usize),
}

#[derive(Clone, Debug)]
pub struct Occurrence(pub Expression, pub Vec<usize>);

pub type Expr = Box<Expression>;

#[derive(Clone, Copy, Debug)]
pub enum Operation {
  Add,
  Sub,
  Mul,
  Div,
}

#[derive(Debug)]
pub struct Arm {
  pub lhs: Pattern,
  pub rhs: Expr,
}

#[derive(Clone, Debug, Default)]
pub enum Pattern {
  #[default]
  Wildcard,
  Variable {
    name: String,
  },
  Number {
    value: i32,
  },
  String {
    value: String,
  },
  Atom {
    value: String,
  },
  Tuple {
    elements: Vec<Pattern>,
  },
}

#[derive(Clone, Debug)]
pub struct FnDefinition {
  pub name: String,
  pub parameters: Vec<String>,
  pub body: Expr,
}

#[derive(Debug)]
pub struct Program {
  pub definitions: BTreeMap<String, FnDefinition>,
}
