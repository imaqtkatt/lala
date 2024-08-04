use std::collections::{BTreeMap, HashMap};

use crate::desugar::{self, Cond, Expression as Desugar, Operation, Tree};

#[derive(Clone)]
pub struct Env {
  fn_definitions: BTreeMap<String, desugar::FnDefinition>,
  variables: HashMap<String, Value>,
}

#[derive(Clone, Debug)]
pub enum Value {
  Number(i32),
  String(String),
  Atom(String),
  Tuple(Vec<Value>),
  Function(Vec<String>, desugar::Expr),
}

impl Env {
  pub fn from_program(program: desugar::Program) -> Self {
    Self {
      fn_definitions: program.definitions,
      variables: HashMap::new(),
    }
  }

  fn fetch(&self, name: &str) -> Result<Value, String> {
    if let Some(value) = self.variables.get(name).cloned() {
      return Ok(value);
    } else {
      self
        .fn_definitions
        .get(name)
        .map(|f| Value::Function(f.parameters.clone(), f.body.clone()))
        .ok_or(format!("Unbound variable {name}"))
    }
  }

  pub fn eval(&mut self, expr: desugar::Expression) -> Result<Value, String> {
    match expr {
      Desugar::Variable { name } => self.fetch(&name),
      Desugar::Number { value } => Ok(Value::Number(value)),
      Desugar::Atom { value } => Ok(Value::Atom(value)),
      Desugar::String { value } => Ok(Value::String(value)),
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
      Desugar::Tuple { elements } => Ok(Value::Tuple(
        elements.into_iter().flat_map(|e| self.eval(e)).collect(),
      )),
      Desugar::Binary { op, lhs, rhs } => match (op, self.eval(*lhs)?, self.eval(*rhs)?) {
        (Operation::Add, Value::Number(a), Value::Number(b)) => Ok(Value::Number(a + b)),
        (Operation::Sub, Value::Number(a), Value::Number(b)) => Ok(Value::Number(a - b)),
        (Operation::Mul, Value::Number(a), Value::Number(b)) => Ok(Value::Number(a * b)),
        (Operation::Div, Value::Number(a), Value::Number(b)) => Ok(Value::Number(a / b)),
        (Operation::Equal, ref x, ref y) if equality(x, y) => Ok(Value::Atom("true".to_string())),
        (Operation::Equal, _, _) => Ok(Value::Atom("false".to_string())),
        _ => Err(format!("Invalid binary operation.")),
      },
      Desugar::Call { callee, arguments } => match self.eval(*callee)? {
        Value::Function(parameters, body) => {
          let mut new_env = self.clone();
          for (x, y) in parameters.into_iter().zip(arguments) {
            new_env.variables.insert(x, self.eval(y)?);
          }
          new_env.eval(*body)
        }
        _ => Err(format!("Expected call to a function definition")),
      },
      Desugar::Access { expr, idx } => match self.eval(*expr)? {
        Value::Tuple(elements) => {
          if let Some(item) = elements.get(idx).cloned() {
            Ok(item)
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
        Value::Atom(ref value) if value == "true" => self.eval(*then_branch),
        _ => self.eval(*else_branch),
      },
    }
  }
}

fn equality(x: &Value, y: &Value) -> bool {
  match (x, y) {
    (Value::Number(a), Value::Number(b)) if a == b => true,
    (Value::String(a), Value::String(b)) if a == b => true,
    (Value::Atom(a), Value::Atom(b)) if a == b => true,
    (Value::Tuple(a), Value::Tuple(b)) if a.len() == b.len() => a
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
            (Cond::Number(a), Value::Number(b)) if a == b => branch.eval(env),
            (Cond::String(a), Value::String(b)) if a == b => branch.eval(env),
            (Cond::Atom(a), Value::Atom(b)) if a == b => branch.eval(env),
            (Cond::Tuple(a), Value::Tuple(b)) if *a == b.len() => branch.eval(env),
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
