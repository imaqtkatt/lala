use crate::{
  ast::{self},
  desugar::{pattern, Expression},
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
    let arity = (&patterns[0]).len();
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

    let (tree, actions) = pattern::Problem::with_parameters(parameters.clone(), patterns, actions);

    Ok(FnDefinition {
      name: self.name,
      parameters,
      body: Box::new(Expression::Match { tree, actions }),
    })
  }
}

// impl ast::FnDefinition {
//   pub fn desugar(self) -> Result<FnDefinition, String> {
//     fn gen_name(gen: usize) -> String {
//       format!("x_{gen}")
//     }

//     let mut patterns: Vec<Vec<_>> = vec![];
//     let mut actions = vec![];
//     for clause in self.clauses {
//       patterns.push(clause.patterns.into_iter().map(|p| p.desugar()).collect());
//       actions.push(clause.body.desugar());
//     }
//     // println!("patterns = {patterns:?}");
//     let arity = (&patterns[0]).len();
//     for pat in patterns.iter() {
//       let curr_arity = pat.len();
//       if curr_arity != arity {
//         return Err(format!(
//           "{}: Arity error, expected {arity} but got {curr_arity}",
//           self.name
//         ));
//       }
//     }

//     let parameters: Vec<String> = (0..arity).map(gen_name).collect();

//     let (tree, actions) = pattern::Problem::with_parameters(parameters.clone(), patterns, actions);

//     Ok(FnDefinition {
//       name: self.name,
//       parameters,
//       body: Box::new(Expression::Match { tree, actions }),
//     })
//   }
// }
