#[derive(Debug)]
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
    scrutinee: Expr,
    arms: Vec<Arm>,
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
}

#[derive(Debug)]
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

#[derive(Debug, Default)]
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

#[derive(Debug)]
pub struct FnDefinition {
  pub name: String,
  pub clauses: Vec<FnClause>,
}

#[derive(Debug)]
pub struct FnClause {
  pub patterns: Vec<Pattern>,
  pub body: Expr,
}

#[derive(Debug)]
pub struct Program {
  pub definitions: Vec<FnDefinition>,
}

pub type Expr = Box<Expression>;
