use std::collections::{BTreeMap, HashMap};

use crate::desugar::{self, Case};

#[derive(Clone)]
pub struct Env {
  fn_definitions: BTreeMap<String, desugar::FnDefinition>,
  variables: HashMap<String, desugar::Expression>,
}

impl Env {
  pub fn from_program(program: desugar::Program) -> Self {
    Self {
      fn_definitions: program.definitions,
      variables: HashMap::new(),
    }
  }

  fn fetch(&self, name: &str) -> Result<desugar::Expression, String> {
    self
      .variables
      .get(name)
      .cloned()
      .ok_or(format!("Unbound variable {name}"))
  }

  pub fn eval(&mut self, expr: desugar::Expression) -> Result<desugar::Expression, String> {
    match expr {
      desugar::Expression::Variable { name } => self.fetch(&name),
      desugar::Expression::Number { .. } => Ok(expr),
      desugar::Expression::Atom { value } => Ok(desugar::Expression::Atom { value }),
      desugar::Expression::String { value } => Ok(desugar::Expression::String { value }),
      desugar::Expression::Let { bind, value, next } => {
        let mut new_env = self.clone();
        new_env.variables.insert(bind, self.eval(*value)?);
        new_env.eval(*next)
      }
      desugar::Expression::Match { ref tree, actions } => {
        let mut curr = tree;
        let idx = 'branch: loop {
          match curr {
            desugar::Tree::Failure => return Err("Match failure".to_string()),
            desugar::Tree::Leaf(leaf) => break leaf,
            desugar::Tree::Switch(ref occ, ref branches, ref default) => {
              let expr = self.eval(occ.clone().to_expression())?;
              'test: for (case, branch) in branches {
                match (case, &expr) {
                  (Case::Number(a), desugar::Expression::Number { value: b }) if a == b => {
                    curr = branch;
                    continue 'branch;
                  }
                  (Case::String(a), desugar::Expression::String { value: b }) if a == b => {
                    curr = branch;
                    continue 'branch;
                  }
                  (Case::Atom(a), desugar::Expression::Atom { value: b }) if a == b => {
                    curr = branch;
                    continue 'branch;
                  }
                  (Case::Tuple(a), desugar::Expression::Tuple { elements: b }) if *a == b.len() => {
                    curr = branch;
                    continue 'branch;
                  }
                  _ => continue 'test,
                }
              }
              curr = default;
              continue 'branch;
            }
          }
        };
        let body = actions[*idx].clone();
        self.eval(body)
      }
      desugar::Expression::Tuple { elements } => Ok(desugar::Expression::Tuple {
        elements: elements.into_iter().flat_map(|e| self.eval(e)).collect(),
      }),
      desugar::Expression::Binary {
        op: _,
        lhs: _,
        rhs: _,
      } => todo!(),
      desugar::Expression::Call { callee, arguments } => {
        let desugar::Expression::Variable { name } = *callee else {
          return Err("Callee must be a variable name".to_string());
        };
        match self.fn_definitions.get(&name).cloned() {
          Some(def) => {
            let mut new_env = self.clone();
            for (x, y) in def.parameters.into_iter().zip(arguments) {
              new_env.variables.insert(x, self.eval(y)?);
            }
            new_env.eval(*def.body)
          }
          None => Err(format!("Unbound function definition {name}")),
        }
      }
      desugar::Expression::Access { expr, idx } => match self.eval(*expr)? {
        desugar::Expression::Tuple { elements } => {
          if let Some(item) = elements.get(idx) {
            self.eval(item.clone())
          } else {
            Err(format!("Index {} out of bounds", idx))
          }
        }
        _ => Err(format!("Accessing not tuple element")),
      },
    }
  }
}
