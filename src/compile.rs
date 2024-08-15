use std::{cell::RefCell, collections::HashMap};

use indexmap::IndexMap;

use crate::desugar::{self, Cond, Expression, Occurrence};

#[derive(Debug)]
pub enum Bytecode {
  Return,
  PushNumber {
    val: i32,
  },
  LoadConstant {
    id: u16,
  },
  GetLocal {
    id: usize,
  },
  SetLocal {
    id: usize,
  },
  TestExact {
    id: u16,
    /// Branch to next or default case.
    branch: usize,
  },
  TestTuple {
    size: usize,
    /// Branch to next or default case.
    branch: usize,
  },
  TestCons {
    branch: usize,
  },
  TestNil {
    branch: usize,
  },
  MakeTuple {
    size: usize,
  },
  GetTuple {
    index: usize,
  },
  GetHd,
  GetTl,
  Jump {
    index: usize,
  },
  MatchFail,
  Undefined,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Constant {
  Number(i32),
  Atom(String),
  String(String),
}

#[derive(Default)]
pub struct Ctx {
  bytecode: Vec<Bytecode>,
  constants: IndexMap<Constant, u16>,
  locals: HashMap<String, usize>,
}

const TEMP_BRANCH: usize = 0;

#[derive(Debug)]
pub struct BytecodeInfo {
  pub bytecode: Vec<Bytecode>,
  pub locals: usize,
  pub constants: IndexMap<Constant, u16>,
}

impl Ctx {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn bytecode(&mut self) -> BytecodeInfo {
    let bytecode = std::mem::take(&mut self.bytecode);
    let locals = self.locals.len();
    self.locals.clear();
    let constants = std::mem::take(&mut self.constants);
    BytecodeInfo {
      bytecode,
      locals,
      constants,
    }
  }

  pub fn fn_definition(&mut self, fun: desugar::FnDefinition) {
    for param in fun.parameters {
      self.make_local(param);
    }
    self.fn_clause(*fun.body);
  }

  pub fn fn_clause(&mut self, expression: Expression) {
    self.compile_expr(expression);
    self.push(Bytecode::Return);
  }

  pub fn push(&mut self, bytecode: Bytecode) -> usize {
    let index = self.bytecode.len();
    self.bytecode.push(bytecode);
    index
  }

  pub fn make_local(&mut self, name: String) -> usize {
    let id = self.locals.len();
    let e = self.locals.entry(name).or_default();
    *e = id;
    *e
  }

  pub fn get_local(&mut self, name: &str) -> usize {
    self.locals[name].clone()
  }

  fn make_constant(&mut self, constant: Constant) -> u16 {
    let id = self.constants.len();
    assert!(id < u16::MAX as usize);
    match self.constants.entry(constant) {
      indexmap::map::Entry::Occupied(o) => *o.get(),
      indexmap::map::Entry::Vacant(v) => *v.insert(id as u16),
    }
  }

  fn compile_occ(&mut self, occurrence: Occurrence) {
    self.compile_expr(occurrence.0);
    for idx in occurrence.1.to_vec() {
      match idx {
        desugar::Acc::Tup(idx) => _ = self.push(Bytecode::GetTuple { index: idx }),
        desugar::Acc::Head => _ = self.push(Bytecode::GetHd),
        desugar::Acc::Tail => _ = self.push(Bytecode::GetTl),
      }
      // self.push(Bytecode::GetTuple { index: idx });
    }
  }

  pub fn compile_cond(&mut self, cond: Cond) -> usize {
    match cond {
      Cond::Cons => self.push(Bytecode::TestCons {
        branch: TEMP_BRANCH,
      }),
      Cond::Nil => self.push(Bytecode::TestNil {
        branch: TEMP_BRANCH,
      }),
      Cond::Number(n) => {
        let id = self.make_constant(Constant::Number(n));
        self.push(Bytecode::TestExact {
          id,
          branch: TEMP_BRANCH,
        })
      }
      Cond::String(s) => {
        let id = self.make_constant(Constant::String(s));
        self.push(Bytecode::TestExact {
          id,
          branch: TEMP_BRANCH,
        })
      }
      Cond::Atom(a) => {
        let id = self.make_constant(Constant::Atom(a));
        self.push(Bytecode::TestExact {
          id,
          branch: TEMP_BRANCH,
        })
      }
      Cond::Tuple(size) => self.push(Bytecode::TestTuple {
        size,
        branch: TEMP_BRANCH,
      }),
    }
  }

  fn compile_case_tree(
    &mut self,
    tree: desugar::Tree,
    actions: Vec<Expression>,
    jumps: &mut Vec<usize>,
  ) {
    match tree {
      desugar::Tree::Failure => {
        self.push(Bytecode::MatchFail);
      }
      desugar::Tree::Leaf(index) => {
        // TODO: don't repeat this compilation
        self.compile_expr(actions[index].clone());
        let idx = self.push(Bytecode::Jump { index: TEMP_BRANCH });
        jumps.push(idx);
      }
      desugar::Tree::Switch(occ, branches, default) => {
        let mut branches = branches.into_iter().peekable();
        while let Some((cond, tree)) = branches.next() {
          self.compile_occ(*occ.clone());
          let cond_location = self.compile_cond(cond);
          self.compile_case_tree(tree, actions.clone(), jumps);
          if let Some(_) = branches.peek() {
            let len = self.bytecode.len();
            match &mut self.bytecode[cond_location] {
              Bytecode::TestExact { branch, .. }
              | Bytecode::TestTuple { branch, .. }
              | Bytecode::TestCons { branch }
              | Bytecode::TestNil { branch } => {
                *branch = len;
              }
              _ => unreachable!(),
            }
          } else {
            let len = self.bytecode.len();
            self.compile_case_tree(*default.clone(), actions.clone(), jumps);
            match &mut self.bytecode[cond_location] {
              Bytecode::TestExact { branch, .. }
              | Bytecode::TestTuple { branch, .. }
              | Bytecode::TestCons { branch }
              | Bytecode::TestNil { branch } => {
                *branch = len;
              }
              _ => unreachable!(),
            }
          }
        }
      }
    }
  }

  pub fn compile_expr(&mut self, expression: Expression) {
    match expression {
      Expression::Variable { ref name } => {
        let id = self.get_local(name);
        self.push(Bytecode::GetLocal { id });
      }
      Expression::Number { value } => {
        self.push(Bytecode::PushNumber { val: value });
      }
      Expression::Atom { value } => {
        let id = self.make_constant(Constant::Atom(value));
        self.push(Bytecode::LoadConstant { id });
      }
      Expression::String { value } => {
        let id = self.make_constant(Constant::String(value));
        self.push(Bytecode::LoadConstant { id });
      }
      Expression::Let { bind, value, next } => {
        self.compile_expr(*value);
        let id = self.make_local(bind);
        self.push(Bytecode::SetLocal { id });
        self.compile_expr(*next);
      }
      Expression::Match { tree, actions } => {
        let mut jumps = Vec::new();
        self.compile_case_tree(tree, actions, &mut jumps);
        let next_bytecode = self.bytecode.len();
        for idx in jumps {
          let Bytecode::Jump { index } = &mut self.bytecode[idx] else {
            unreachable!()
          };
          *index = next_bytecode;
        }
      }
      Expression::Tuple { elements } => {
        let size = elements.len();
        for element in elements.into_iter() {
          self.compile_expr(element);
        }
        self.push(Bytecode::MakeTuple { size });
      }
      Expression::Binary {
        op: _,
        lhs: _,
        rhs: _,
      } => todo!(),
      Expression::Call {
        callee: _,
        arguments: _,
      } => todo!(),
      Expression::If {
        condition,
        then_branch,
        else_branch,
      } => {
        self.compile_expr(*condition);

        let id = self.make_constant(Constant::Atom("true".to_string()));
        self.push(Bytecode::LoadConstant { id });
        let branch = self.push(Bytecode::TestExact { id, branch: 0 });

        self.compile_expr(*else_branch);
        let len = self.bytecode.len();
        let Bytecode::TestExact { branch, .. } = &mut self.bytecode[branch] else {
          unreachable!()
        };
        *branch = len;
        self.compile_expr(*then_branch);
      }
      Expression::Access { expr, idx } => {
        self.compile_expr(*expr);
        match idx {
          desugar::Acc::Tup(idx) => _ = self.push(Bytecode::GetTuple { index: idx }),
          desugar::Acc::Head => _ = self.push(Bytecode::GetHd),
          desugar::Acc::Tail => _ = self.push(Bytecode::GetTl),
        }
      }
      Expression::Cons { hd: _, tl: _ } => _ = self.push(Bytecode::Undefined),
      Expression::Nil => _ = self.push(Bytecode::Undefined),
    }
  }
}

#[derive(Clone, Debug)]
pub enum Value {
  Number(i32),
  Tuple(Vec<Value>),
  Atom(String),
  String(String),
}

impl Default for Value {
  fn default() -> Self {
    Self::Number(0)
  }
}

pub struct Machine<'a> {
  code: &'a [Bytecode],
  ip: RefCell<usize>,
  constants: Vec<Constant>,
}

impl<'a> Machine<'a> {
  pub fn new(info: &'a BytecodeInfo) -> Self {
    Self {
      code: &info.bytecode,
      ip: RefCell::new(0),
      constants: info.constants.keys().cloned().collect(),
    }
  }

  fn load_constant(&self, id: u16) -> Value {
    let c = self.constants[id as usize].clone();
    match c {
      Constant::Number(n) => Value::Number(n),
      Constant::Atom(a) => Value::Atom(a),
      Constant::String(s) => Value::String(s),
    }
  }

  pub fn run(&mut self, stack: &mut Vec<Value>, mut locals: Vec<Value>) {
    loop {
      let ins = self.fetch();
      println!("ins = {ins:?}");
      match ins {
        Bytecode::Return => break,
        Bytecode::PushNumber { val } => {
          stack.push(Value::Number(*val));
        }
        Bytecode::LoadConstant { id } => {
          let c = self.load_constant(*id);
          stack.push(c);
        }
        Bytecode::GetLocal { id } => {
          let a = locals[*id].clone();
          stack.push(a);
        }
        Bytecode::SetLocal { id } => {
          let a = stack.pop().unwrap();
          locals[*id] = a;
        }
        Bytecode::TestExact { id, branch } => {
          match (stack.pop().unwrap(), self.load_constant(*id)) {
            (Value::Number(a), Value::Number(b)) if a == b => {}
            (Value::Atom(ref a), Value::Atom(ref b)) if a == b => {}
            (Value::String(ref a), Value::String(ref b)) if a == b => {}
            _ => *self.ip.borrow_mut() = *branch,
          }
        }
        Bytecode::TestTuple { size, branch } => match (stack.pop().unwrap(), *size) {
          (Value::Tuple(x), y) if x.len() == y => {}
          _ => *self.ip.borrow_mut() = *branch,
        },
        Bytecode::TestCons { branch } => todo!(),
        Bytecode::TestNil { branch } => todo!(),
        Bytecode::MakeTuple { size } => {
          let mut s = Vec::with_capacity(*size);
          for _ in 0..*size {
            s.insert(0, stack.pop().unwrap());
          }
          stack.push(Value::Tuple(s));
        }
        Bytecode::GetTuple { index } => {
          let Value::Tuple(t) = stack.pop().unwrap() else {
            unreachable!()
          };
          stack.push(t[*index].clone());
        }
        Bytecode::Jump { index } => *self.ip.borrow_mut() = *index,
        Bytecode::MatchFail => panic!("Match failure"),
        Bytecode::GetHd => todo!(),
        Bytecode::GetTl => todo!(),
        Bytecode::Undefined => todo!(),
      }
    }
  }

  fn fetch(&self) -> &Bytecode {
    let mut ip = self.ip.borrow_mut();
    let r = &self.code[*ip];
    *ip += 1;
    r
  }
}

#[cfg(test)]
mod test {
  use crate::{desugar::Desugar, lexer::Lexer, parser::Parser};

  use super::{Ctx, Machine, Value};

  #[test]
  fn test_compile() {
    //     let src = r#"
    // let x = {1, 99} in
    // case x of
    //   {1, 1} -> 42;
    //   {1, x} -> x;
    //   "oi"   -> "tchau";
    //   _      -> 69
    // end
    // "#;
    let src = r#"
case [1, 2] of
  [2, 3] -> 4;
  _ -> 0
end
"#;
    let mut parser = Parser::new(Lexer::new(src));
    let expr = parser.expression().unwrap();
    let expr = expr.desugar().unwrap();
    // println!("{expr:?}");
    let mut ctx = Ctx::new();
    ctx.fn_clause(expr);

    let info = ctx.bytecode();

    for (idx, b) in info.bytecode.iter().enumerate() {
      println!("{idx}: {b:?}");
    }
    for (c, id) in info.constants.iter() {
      println!("{c:?}: {id}");
    }
    println!("locals = {}", info.locals);

    let mut machine = Machine::new(&info);
    let mut stack = vec![];
    let locals = vec![Value::default(); info.locals + 1];
    machine.run(&mut stack, locals);
    println!("{stack:?}");
  }
}
