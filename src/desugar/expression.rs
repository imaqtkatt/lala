use crate::ast;

use super::{
  pattern::{self},
  Desugar, Expression, Operation,
};

impl Desugar for ast::Expression {
  type Out = Expression;

  type Err = ();

  fn desugar(self) -> Result<Self::Out, Self::Err> {
    match self {
      ast::Expression::Variable { name } => Ok(Expression::Variable { name }),
      ast::Expression::Number { value } => Ok(Expression::Number { value }),
      ast::Expression::Atom { value } => Ok(Expression::Atom { value }),
      ast::Expression::String { value } => Ok(Expression::String { value }),
      ast::Expression::Let { bind, value, next } => Ok(Expression::Let {
        bind,
        value: value.desugar()?.into(),
        next: next.desugar()?.into(),
      }),
      ast::Expression::Match { scrutinee, arms } => {
        let mut left = vec![];
        let mut actions = vec![];
        for arm in arms.into_iter() {
          left.push(arm.lhs.into_iter().map(|p| p.desugar()).collect());
          actions.push(arm.rhs.desugar()?);
        }
        let scrutinee = scrutinee.into_iter().flat_map(|s| s.desugar()).collect();
        Ok(pattern::Problem::compile(scrutinee, left, actions))
      }
      ast::Expression::Tuple { elements } => Ok(Expression::Tuple {
        elements: elements.into_iter().flat_map(|e| e.desugar()).collect(),
      }),
      ast::Expression::Binary { op, lhs, rhs } => Ok(Expression::Binary {
        op: op.desugar(),
        lhs: lhs.desugar()?.into(),
        rhs: rhs.desugar()?.into(),
      }),
      ast::Expression::Call { callee, arguments } => Ok(Expression::Call {
        callee: callee.desugar()?.into(),
        arguments: arguments.into_iter().flat_map(|a| a.desugar()).collect(),
      }),
      ast::Expression::If {
        condition,
        then_branch,
        else_branch,
      } => Ok(Expression::If {
        condition: condition.desugar()?.into(),
        then_branch: then_branch.desugar()?.into(),
        else_branch: else_branch.desugar()?.into(),
      }),
      ast::Expression::List { elements } => Ok(
        elements
          .into_iter()
          .flat_map(|e| e.desugar())
          .rfold(Expression::Nil, |acc, nxt| Expression::Cons {
            hd: Box::new(nxt),
            tl: Box::new(acc),
          }),
      ),
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
      ast::Operation::Equal => Operation::Equal,
    }
  }
}
