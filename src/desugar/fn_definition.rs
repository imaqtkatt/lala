use crate::{
  ast::{self},
  desugar::{pattern, Expression},
};

use super::FnDefinition;

impl ast::FnDefinition {
  pub fn desugar(self) -> Result<FnDefinition, String> {
    fn gen_name(gen: usize) -> String {
      format!("a_{gen}")
    }

    let mut patterns: Vec<Vec<_>> = vec![];
    let mut actions = vec![];
    for clause in self.clauses {
      patterns.push(clause.patterns.into_iter().map(|p| p.desugar()).collect());
      actions.push(clause.body.desugar());
    }
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

    let scrutinee = parameters
      .iter()
      .map(|name| Expression::Variable { name: name.clone() })
      .collect();
    let body = pattern::compile_match(scrutinee, patterns, actions);

    Ok(FnDefinition {
      name: self.name,
      parameters,
      body: Box::new(body),
    })
  }
}
