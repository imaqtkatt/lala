use std::collections::BTreeMap;

use crate::ast;

use super::{Desugar, Program};

impl Desugar for ast::Program {
  type Out = Program;

  type Err = String;

  fn desugar(self) -> Result<Self::Out, Self::Err> {
    let mut definitions = BTreeMap::new();
    for definition in self.definitions {
      definitions.insert(definition.name.clone(), definition.desugar()?);
    }
    Ok(Program { definitions })
  }
}
