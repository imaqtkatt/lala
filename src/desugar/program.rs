use std::collections::BTreeMap;

use crate::ast;

use super::Program;

impl ast::Program {
  pub fn desugar(self) -> Result<Program, String> {
    let mut definitions = BTreeMap::new();
    for definition in self.definitions {
      definitions.insert(definition.name.clone(), definition.desugar()?);
    }
    Ok(Program { definitions })
  }
}
