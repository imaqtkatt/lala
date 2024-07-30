use crate::ast;

use super::{pattern, Expression, Operation};

impl ast::Expression {
  pub fn desugar(self) -> Expression {
    match self {
      ast::Expression::Variable { name } => Expression::Variable { name },
      ast::Expression::Number { value } => Expression::Number { value },
      ast::Expression::Atom { value } => Expression::Atom { value },
      ast::Expression::String { value } => Expression::String { value },
      ast::Expression::Let { bind, value, next } => Expression::Let {
        bind,
        value: value.desugar().into(),
        next: next.desugar().into(),
      },
      ast::Expression::Match { scrutinee, arms } => {
        let mut left = vec![];
        let mut actions = vec![];
        for arm in arms.into_iter() {
          left.push(vec![arm.lhs.desugar()]);
          actions.push(arm.rhs.desugar());
        }
        pattern::compile_match(vec![scrutinee.desugar()], left, actions)
      }
      ast::Expression::Tuple { elements } => Expression::Tuple {
        elements: elements.into_iter().map(|e| e.desugar()).collect(),
      },
      ast::Expression::Binary { op, lhs, rhs } => Expression::Binary {
        op: op.desugar(),
        lhs: lhs.desugar().into(),
        rhs: rhs.desugar().into(),
      },
      ast::Expression::Call { callee, arguments } => Expression::Call {
        callee: callee.desugar().into(),
        arguments: arguments.into_iter().map(|a| a.desugar()).collect(),
      },
    }
  }
}

impl ast::Operation {
  pub fn desugar(self) -> Operation {
    match self {
      ast::Operation::Add => Operation::Add,
      ast::Operation::Sub => Operation::Sub,
      ast::Operation::Mul => Operation::Mul,
      ast::Operation::Div => Operation::Div,
    }
  }
}
