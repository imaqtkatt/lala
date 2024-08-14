use crate::{
  ast::{self},
  desugar::{self, pattern, Expression, Pattern},
};

use super::{Desugar, FnDefinition};

impl Desugar for ast::FnDefinition {
  type Out = FnDefinition;

  type Err = String;

  fn desugar(self) -> Result<Self::Out, Self::Err> {
    fn gen_name(gen: usize) -> String {
      format!("x_{gen}")
    }

    let mut patterns: Vec<Vec<_>> = vec![];
    let mut actions = vec![];
    for clause in self.clauses {
      patterns.push(clause.patterns.into_iter().map(|p| p.desugar()).collect());
      actions.push(
        clause
          .body
          .desugar()
          .map_err(|_| format!("Could not desugar clause body"))?,
      );
    }
    // println!("patterns = {patterns:?}");
    assert!(patterns.len() > 0 && actions.len() > 0);
    let arity = (&patterns[0]).len();
    if patterns.len() == 1
      && patterns[0]
        .iter()
        .all(|p| matches!(p, Pattern::Variable { .. } | Pattern::Wildcard))
    {
      let parameters: Vec<String> = (0..arity).map(gen_name).collect();

      let body = Box::new(actions.into_iter().nth(0).unwrap());

      Ok(FnDefinition {
        name: self.name,
        parameters,
        body,
      })
    } else {
      for pat in patterns.iter() {
        let curr_arity = pat.len();
        if curr_arity != arity {
          return Err(format!(
            "{}: Arity error, expected {arity} but got {curr_arity}",
            self.name
          ));
        }
      }

      let parameters: Vec<String> = (0..arity).map(gen_name).collect();

      let (tree, actions) =
        pattern::Problem::with_parameters(parameters.clone(), patterns, actions);

      Ok(FnDefinition {
        name: self.name,
        parameters,
        body: Box::new(Expression::Match { tree, actions }),
      })
    }
  }
}
