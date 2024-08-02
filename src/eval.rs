use std::collections::{BTreeMap, HashMap};

use crate::desugar::{self, Cond, Expression as Desugar, Operation, Tree};

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
      Desugar::Variable { name } => self.fetch(&name),
      Desugar::Number { .. } => Ok(expr),
      Desugar::Atom { value } => Ok(Desugar::Atom { value }),
      Desugar::String { value } => Ok(Desugar::String { value }),
      Desugar::Let { bind, value, next } => {
        let mut new_env = self.clone();
        new_env.variables.insert(bind, self.eval(*value)?);
        new_env.eval(*next)
      }
      Desugar::Match { ref tree, actions } => {
        let idx = tree.eval(self)?;
        let body = actions[idx].clone();
        self.eval(body)
      }
      Desugar::Tuple { elements } => Ok(Desugar::Tuple {
        elements: elements.into_iter().flat_map(|e| self.eval(e)).collect(),
      }),
      Desugar::Binary { op, lhs, rhs } => match (op, self.eval(*lhs)?, self.eval(*rhs)?) {
        (Operation::Add, Desugar::Number { value: a }, Desugar::Number { value: b }) => {
          Ok(Desugar::Number { value: a + b })
        }
        (Operation::Sub, Desugar::Number { value: a }, Desugar::Number { value: b }) => {
          Ok(Desugar::Number { value: a - b })
        }
        (Operation::Mul, Desugar::Number { value: a }, Desugar::Number { value: b }) => {
          Ok(Desugar::Number { value: a * b })
        }
        (Operation::Div, Desugar::Number { value: a }, Desugar::Number { value: b }) => {
          Ok(Desugar::Number { value: a / b })
        }
        (Operation::Equal, x, y) => {
          if equality(x, y) {
            Ok(Desugar::Atom {
              value: "true".to_string(),
            })
          } else {
            Ok(Desugar::Atom {
              value: "false".to_string(),
            })
          }
        }
        _ => Err(format!("Invalid binary operation.")),
      },
      Desugar::Call { callee, arguments } => {
        let Desugar::Variable { name } = *callee else {
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
      Desugar::Access { expr, idx } => match self.eval(*expr)? {
        Desugar::Tuple { elements } => {
          if let Some(item) = elements.get(idx) {
            self.eval(item.clone())
          } else {
            Err(format!("Index {} out of bounds", idx))
          }
        }
        _ => Err(format!("Accessing not tuple element")),
      },
      Desugar::If {
        condition,
        then_branch,
        else_branch,
      } => match self.eval(*condition)? {
        Desugar::Atom { ref value } if value == "true" => self.eval(*then_branch),
        _ => self.eval(*else_branch),
      },
    }
  }
}

fn equality(x: Desugar, y: Desugar) -> bool {
  match (x, y) {
    (Desugar::Number { value: a }, Desugar::Number { value: b }) if a == b => true,
    (Desugar::String { value: a }, Desugar::String { value: b }) if a == b => true,
    (Desugar::Atom { value: a }, Desugar::Atom { value: b }) if a == b => true,
    (Desugar::Tuple { elements: a }, Desugar::Tuple { elements: b }) if a.len() == b.len() => a
      .into_iter()
      .zip(b.into_iter())
      .all(|(x, y)| equality(x, y)),
    _ => false,
  }
}

impl Tree {
  pub fn eval(&self, env: &mut Env) -> Result<usize, String> {
    match self {
      Tree::Failure => Err("Match failure".to_string()),
      Tree::Leaf(idx) => Ok(*idx),
      Tree::Switch(occ, branches, default) => {
        let expr = env.eval(occ.clone().to_expression())?;
        for (case, branch) in branches {
          let res = match (case, &expr) {
            (Cond::Number(a), Desugar::Number { value: b }) if a == b => branch.eval(env),
            (Cond::String(a), Desugar::String { value: b }) if a == b => branch.eval(env),
            (Cond::Atom(a), Desugar::Atom { value: b }) if a == b => branch.eval(env),
            (Cond::Tuple(a), Desugar::Tuple { elements: b }) if *a == b.len() => branch.eval(env),
            _ => continue,
          };
          match res {
            Ok(leaf) => return Ok(leaf),
            Err(_) => return default.eval(env),
          }
        }
        default.eval(env)
      }
    }
  }
}
