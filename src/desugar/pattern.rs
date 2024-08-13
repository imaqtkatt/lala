use std::collections::{BTreeSet, VecDeque};

use crate::{
  ast::{self},
  desugar::Expression,
};

use super::{Cond, Occurrence, Pattern, Tree};

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

impl Pattern {
  pub fn binders(&self, e: Expression) -> Vec<(String, Occurrence)> {
    fn bind(pat: &Pattern, occ: Occurrence, binders: &mut Vec<(String, Occurrence)>) {
      match pat {
        Pattern::Variable { name } => binders.push((name.clone(), occ)),
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
    let mut binders = vec![];
    bind(self, Occurrence(e, vec![]), &mut binders);
    binders
  }
}

pub type Matrix = Vec<Row>;

impl Occurrence {
  pub fn with_index(&self, idx: usize) -> Self {
    let mut indexes = self.1.clone();
    indexes.push(idx);
    Self(self.0.clone(), indexes)
  }
}

#[derive(Clone)]
pub struct Case {
  occurrence: Occurrence,
  pattern: Pattern,
}

impl Case {
  fn derive(occurrence: &Occurrence, pattern: Pattern, idx: usize) -> Self {
    Self {
      occurrence: occurrence.with_index(idx),
      pattern,
    }
  }

  fn wildcard(occurrence: &Occurrence) -> Self {
    Self {
      occurrence: occurrence.clone(),
      pattern: Pattern::Wildcard,
    }
  }

  pub fn specialize(self, cond: Cond) -> Option<VecDeque<Case>> {
    match (self.pattern, cond) {
      (Pattern::Number { value: a }, Cond::Number(b)) if a == b => Some(VecDeque::new()),
      (Pattern::Atom { value: a }, Cond::Atom(b)) if a == b => Some(VecDeque::new()),
      (Pattern::String { value: a }, Cond::String(b)) if a == b => Some(VecDeque::new()),
      (Pattern::Tuple { elements: a }, Cond::Tuple(b)) if a.len() == b => Some(
        a.into_iter()
          .enumerate()
          .map(|(idx, pat)| Self::derive(&self.occurrence, pat, idx))
          .collect(),
      ),
      (Pattern::Variable { .. } | Pattern::Wildcard, Cond::Number(_)) => Some(VecDeque::new()),
      (Pattern::Variable { .. } | Pattern::Wildcard, Cond::Tuple(b)) => {
        Some((0..b).map(|_| Case::wildcard(&self.occurrence)).collect())
      }
      _ => None,
    }
  }

  pub fn default(self) -> Option<VecDeque<Case>> {
    match self.pattern {
      Pattern::Variable { .. } | Pattern::Wildcard => Some(VecDeque::new()),
      _ => None,
    }
  }

  pub fn as_cond(&self) -> Option<Cond> {
    match &self.pattern {
      Pattern::Variable { .. } | Pattern::Wildcard => None,
      Pattern::Number { value: n } => Some(Cond::Number(*n)),
      Pattern::Tuple { elements: pats } => Some(Cond::Tuple(pats.len())),
      Pattern::Atom { value } => Some(Cond::Atom(value.clone())),
      Pattern::String { value } => Some(Cond::String(value.clone())),
    }
  }
}

#[derive(Clone)]
pub struct Row(pub VecDeque<Case>, pub usize);

impl Row {
  pub fn default(mut self) -> Option<Self> {
    let head = self.0.pop_front()?;
    let mut tail = self.0;
    let mut new_head = head.default()?;
    new_head.append(&mut tail);

    Some(Row(new_head, self.1))
  }

  pub fn specialize(mut self, cond: Cond) -> Option<Self> {
    let head = self.0.pop_front()?;
    let mut tail = self.0;
    let mut new_head = head.specialize(cond)?;
    new_head.append(&mut tail);

    Some(Row(new_head, self.1))
  }

  pub fn head_cond(&self) -> Option<Cond> {
    self.0.front()?.as_cond()
  }
}

pub struct Problem {
  matrix: Matrix,
}

impl Problem {
  pub fn compile(
    scrutinee: Vec<Expression>,
    patterns: Vec<Vec<Pattern>>,
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
    let (tree, actions) = Problem::with_parameters(names.clone(), patterns, actions);
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

  pub fn with_parameters(
    parameters: Vec<String>,
    patterns: Vec<Vec<Pattern>>,
    actions: Vec<Expression>,
  ) -> (Tree, Vec<Expression>) {
    let mut new_actions = vec![];

    for (pats, mut action) in patterns.iter().zip(actions) {
      let mut ctx = vec![];
      for (scrutinee, pat) in parameters.iter().zip(pats) {
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

    let scrutinee = parameters
      .iter()
      .map(|name| Expression::Variable { name: name.clone() })
      .collect();
    let tree = Problem::new(patterns, scrutinee).derive();
    (tree, new_actions)
  }

  pub fn new(patterns: Vec<Vec<Pattern>>, scrutinee: Vec<Expression>) -> Self {
    let matrix = patterns
      .into_iter()
      .enumerate()
      .map(|(idx, patterns)| {
        let mut row = VecDeque::new();
        for (idx, pattern) in patterns.into_iter().enumerate() {
          row.push_back(Case {
            occurrence: Occurrence(scrutinee[idx].clone(), vec![]),
            pattern,
          })
        }
        Row(row, idx)
      })
      .collect();
    Problem { matrix }
  }

  pub fn matching_leaf(&self) -> Option<usize> {
    let head = self.matrix.first()?;

    if head.0.is_empty() {
      Some(head.1)
    } else {
      None
    }
  }

  pub fn head_occurrence(&self) -> Occurrence {
    self.matrix[0].0[0].occurrence.clone()
  }

  pub fn head_conds(&self) -> BTreeSet<Cond> {
    let mut conds = BTreeSet::new();

    for row in &self.matrix {
      if let Some(cond) = row.head_cond() {
        conds.insert(cond);
      }
    }

    conds
  }

  pub fn default(self) -> Tree {
    let matrix = self
      .matrix
      .into_iter()
      .filter_map(|row| row.default())
      .collect();
    Problem { matrix }.derive()
  }

  pub fn specialize(&self, cond: Cond) -> Tree {
    let matrix = self
      .matrix
      .iter()
      .filter_map(|row| row.clone().specialize(cond.clone()))
      .collect();
    Problem { matrix }.derive()
  }

  pub fn derive(self) -> Tree {
    if self.matrix.is_empty() {
      Tree::Failure
    } else if let Some(leaf) = self.matching_leaf() {
      Tree::Leaf(leaf)
    } else {
      let occurrence = self.head_occurrence();
      let mut cases = vec![];
      let conds = self.head_conds();

      for cond in conds {
        cases.push((cond.clone(), self.specialize(cond)));
      }

      let default = Box::new(self.default());

      if cases.is_empty() {
        *default
      } else {
        Tree::Switch(Box::new(occurrence), cases, default)
      }
    }
  }
}
