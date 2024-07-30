use std::collections::{BTreeSet, HashMap};

use crate::{ast, desugar::Expression};

use super::{Case, Occurrence, Pattern, Tree};

#[derive(Clone)]
pub struct Row(Vec<Pattern>);

impl Row {
  pub fn is_irrefutable(&self) -> bool {
    self
      .0
      .iter()
      .all(|x| matches!(x, Pattern::Variable { .. } | Pattern::Wildcard))
  }
}

pub type Matrix = Vec<Row>;

impl Occurrence {
  pub fn with_index(&self, idx: usize) -> Self {
    let mut indexes = self.1.clone();
    indexes.push(idx);
    Self(self.0.clone(), indexes)
  }

  pub fn specialize(&self, case: Case) -> Vec<Self> {
    match case {
      Case::Number(_) | Case::String(_) | Case::Atom(_) => vec![],
      Case::Tuple(size) => (0..size).map(|x| self.with_index(x)).collect(),
    }
  }
}

pub type Actions = Vec<usize>;

fn find_refutable(matrix: &Matrix) -> usize {
  for (i, row) in matrix.iter().enumerate() {
    for j in row.0.iter() {
      if matches!(
        j,
        Pattern::Number { .. }
          | Pattern::String { .. }
          | Pattern::Tuple { .. }
          | Pattern::Atom { .. }
      ) {
        return i;
      }
    }
  }

  panic!("Should not happen")
}

fn swap_matrix(matrix: &mut Matrix, idx: usize) {
  matrix.iter_mut().for_each(|row| row.0.swap(0, idx))
}

fn swap_actions(actions: &mut Actions, idx: usize) {
  actions.swap(0, idx);
}

fn head_cases(matrix: &Matrix) -> BTreeSet<Case> {
  let mut heads = BTreeSet::new();

  for row in matrix.iter() {
    match &row.0[0] {
      Pattern::Number { value } => {
        heads.insert(Case::Number(*value));
      }
      Pattern::String { value } => {
        heads.insert(Case::String(value.clone()));
      }
      Pattern::Atom { value } => {
        heads.insert(Case::Atom(value.clone()));
      }
      Pattern::Tuple { elements } => {
        heads.insert(Case::Tuple(elements.len()));
      }
      Pattern::Variable { .. } | Pattern::Wildcard => {}
    }
  }

  heads
}

impl Pattern {
  pub fn binders(&self, e: Expression) -> HashMap<String, Occurrence> {
    fn bind(pat: &Pattern, occ: Occurrence, binders: &mut HashMap<String, Occurrence>) {
      match pat {
        Pattern::Variable { name } => _ = binders.insert(name.clone(), occ),
        Pattern::Tuple { elements } => {
          for (i, e) in elements.iter().enumerate() {
            bind(e, occ.with_index(i), binders);
          }
        }
        Pattern::Wildcard
        | Pattern::Number { .. }
        | Pattern::String { .. }
        | Pattern::Atom { .. } => (),
      }
    }
    let mut binders = HashMap::new();
    bind(self, Occurrence(e, vec![]), &mut binders);
    binders
  }
}

impl Row {
  pub fn specialize(&self, case: Case) -> Option<Row> {
    match (case, &self.0[0]) {
      (_, Pattern::Variable { .. } | Pattern::Wildcard) => None,
      (Case::Number(a), Pattern::Number { value: b }) if a == *b => Some(Row(self.0[1..].to_vec())),
      (Case::String(a), Pattern::String { value: b }) if a == *b => Some(Row(self.0[1..].to_vec())),
      (Case::Atom(a), Pattern::Atom { value: b }) if a == *b => Some(Row(self.0[1..].to_vec())),
      (Case::Tuple(size), Pattern::Tuple { elements: pats }) if size == pats.len() => {
        let mut row = self.0[1..].to_vec();
        row.extend(pats.to_vec());
        Some(Row(row))
      }
      _ => None,
    }
  }

  pub fn default(&self) -> Option<Row> {
    match &self.0[0] {
      Pattern::Variable { .. } | Pattern::Wildcard => Some(Row(self.0[1..].to_vec())),
      _ => None,
    }
  }
}

fn specialize_matrix(
  matrix: &Matrix,
  case: Case,
  actions: &Actions,
  occurrences: &Vec<Occurrence>,
) -> (Matrix, Actions, Vec<Occurrence>) {
  let mut new_matrix = Vec::new();
  let mut new_actions = Vec::new();

  for (i, row) in matrix.iter().cloned().enumerate() {
    if let Some(row) = row.specialize(case.clone()) {
      new_matrix.push(row);
      new_actions.push(actions[i]);
    }
  }

  let mut occ = occurrences[0].specialize(case.clone());
  occ.extend(occurrences.iter().skip(1).cloned());
  (new_matrix, new_actions, occ)
}

fn default_matrix(matrix: &Matrix, actions: &Actions) -> (Matrix, Actions) {
  let mut new_matrix = Vec::new();
  let mut new_actions = Vec::new();

  for (i, row) in matrix.iter().cloned().enumerate() {
    if let Some(row) = row.default() {
      new_matrix.push(row);
      new_actions.push(actions[i]);
    }
  }

  (new_matrix, new_actions)
}

fn swap_occurrences(occurrences: &mut Vec<Occurrence>, refutable: usize) {
  occurrences.swap(0, refutable);
}

pub fn gen_case_tree(
  mut matrix: Matrix,
  mut actions: Actions,
  mut occurrences: Vec<Occurrence>,
) -> Tree {
  if matrix.is_empty() {
    Tree::Failure
  } else if matrix[0].is_irrefutable() {
    Tree::Leaf(actions[0])
  } else {
    let refutable = find_refutable(&matrix);
    swap_matrix(&mut matrix, refutable);
    swap_actions(&mut actions, refutable);
    swap_occurrences(&mut occurrences, refutable);
    let heads = head_cases(&matrix);
    let default_case = {
      let (matrix, actions) = default_matrix(&matrix, &actions);
      Box::new(gen_case_tree(matrix, actions, occurrences.clone()))
    };

    let mut branches = Vec::new();

    for head in heads {
      let (matrix, actions, occurrences) =
        specialize_matrix(&matrix, head.clone(), &actions, &occurrences);
      branches.push((head, gen_case_tree(matrix, actions, occurrences)))
    }

    Tree::Switch(Box::new(occurrences[0].clone()), branches, default_case)
  }
}

impl Tree {
  pub fn compile(scrutinee: Vec<Expression>, matrix: Vec<Vec<Pattern>>) -> Self {
    let actions = (0..matrix.len()).collect();
    let matrix = matrix.into_iter().map(Row).collect();
    let occurrences = scrutinee
      .into_iter()
      .map(|x| Occurrence(x, vec![]))
      .collect();
    gen_case_tree(matrix, actions, occurrences)
  }
}

fn compile_match_with_names(
  scrutinee_names: Vec<String>,
  left: Vec<Vec<Pattern>>,
  actions: Vec<Expression>,
) -> (Tree, Vec<Expression>) {
  let mut new_actions = vec![];

  for (pats, mut action) in left.iter().zip(actions) {
    let mut ctx = vec![];
    for (scrutinee, pat) in scrutinee_names.iter().zip(pats) {
      let e = Expression::Variable {
        name: scrutinee.clone(),
      };
      for (binder, name) in pat.binders(e) {
        ctx.push((binder, name.to_expression()));
      }
    }
    action = ctx
      .into_iter()
      .fold(action, |acc, (binder, name)| Expression::Let {
        bind: binder,
        value: name.into(),
        next: acc.into(),
      });
    new_actions.push(action);
  }

  let scrutinee = scrutinee_names
    .into_iter()
    .map(|name| Expression::Variable { name })
    .collect();
  let tree = Tree::compile(scrutinee, left);
  (tree, new_actions)
}

pub fn compile_match(
  scrutinee: Vec<Expression>,
  left: Vec<Vec<Pattern>>,
  actions: Vec<Expression>,
) -> Expression {
  fn gen_scrutinee_name(e: &Expression, gen: &mut usize) -> (String, bool) {
    *gen += 1;
    match e {
      Expression::Variable { name } => (name.clone(), false),
      _ => (format!("a_{gen}"), true),
    }
  }

  let mut gen = 0;
  let (names, need_let): (Vec<_>, Vec<_>) = scrutinee
    .iter()
    .map(|e| gen_scrutinee_name(e, &mut gen))
    .unzip();
  let (tree, actions) = compile_match_with_names(names.clone(), left, actions);
  names.iter().zip(need_let).zip(scrutinee).fold(
    Expression::Match { tree, actions },
    |x, ((name, need_let), scrutinee)| {
      if need_let {
        Expression::Let {
          bind: name.clone(),
          value: Box::new(scrutinee),
          next: Box::new(x),
        }
      } else {
        x
      }
    },
  )
}

impl Occurrence {
  pub fn to_expression(self) -> Expression {
    self
      .1
      .into_iter()
      .fold(self.0, |acc, nxt| Expression::Access {
        expr: Box::new(acc),
        idx: nxt,
      })
  }
}

impl ast::Pattern {
  pub fn desugar(self) -> Pattern {
    match self {
      ast::Pattern::Wildcard => Pattern::Wildcard,
      ast::Pattern::Variable { name } => Pattern::Variable { name },
      ast::Pattern::Number { value } => Pattern::Number { value },
      ast::Pattern::String { value } => Pattern::String { value },
      ast::Pattern::Atom { value } => Pattern::Atom { value },
      ast::Pattern::Tuple { elements } => Pattern::Tuple {
        elements: elements.into_iter().map(|e| e.desugar()).collect(),
      },
    }
  }
}
