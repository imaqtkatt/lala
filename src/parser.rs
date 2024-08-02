use crate::{
  ast::{Arm, Expression, FnClause, FnDefinition, Operation, Pattern, Program},
  lexer::{Lexer, Token, TokenKind},
};

pub struct Parser<'input> {
  lexer: Lexer<'input>,
  curr: Token,
  next: Token,
}

const PREC: &[&[TokenKind]] = &[
  &[TokenKind::DoubleEquals],
  &[TokenKind::Plus, TokenKind::Minus],
  &[TokenKind::Star, TokenKind::Slash],
];

impl<'input> Parser<'input> {
  pub fn new(mut lexer: Lexer<'input>) -> Self {
    Self {
      curr: lexer.next_token(),
      next: lexer.next_token(),
      lexer,
    }
  }

  pub fn eat(&mut self) -> Token {
    let new_curr = std::mem::replace(&mut self.next, self.lexer.next_token());
    std::mem::replace(&mut self.curr, new_curr)
  }

  pub fn expect(&mut self, kind: TokenKind) -> Result<Token, String> {
    if self.curr.kind == kind {
      Ok(self.eat())
    } else {
      Err(format!("Expected {kind:?} but got {:?}", self.curr.lexeme))
    }
  }

  fn is(&self, kind: TokenKind) -> bool {
    self.curr.kind == kind
  }

  fn kind(&self) -> TokenKind {
    self.curr.kind
  }

  pub fn primary(&mut self) -> Result<Expression, String> {
    match self.kind() {
      TokenKind::Atom => self.atom(),
      TokenKind::Number => self.number(),
      TokenKind::Identifier => self.variable(),
      TokenKind::String => self.string(),
      TokenKind::LParens => {
        self.expect(TokenKind::LParens)?;
        let expr = self.expression()?;
        self.expect(TokenKind::RParens)?;
        Ok(expr)
      }
      TokenKind::LBrace => {
        self.expect(TokenKind::LBrace)?;
        let mut elements = vec![];
        while !self.is(TokenKind::RBrace) {
          elements.push(self.expression()?);
          if self.is(TokenKind::RBrace) {
            break;
          }
          self.expect(TokenKind::Comma)?;
        }
        self.expect(TokenKind::RBrace)?;
        Ok(Expression::Tuple { elements })
      }
      _ => Err("Expected primary expression".to_string()),
    }
  }

  fn atom(&mut self) -> Result<Expression, String> {
    self.expect(TokenKind::Atom).map(|token| Expression::Atom {
      value: token.lexeme,
    })
  }

  fn number(&mut self) -> Result<Expression, String> {
    self
      .expect(TokenKind::Number)
      .map(|token| Expression::Number {
        value: token.lexeme.parse().unwrap(),
      })
  }

  fn variable(&mut self) -> Result<Expression, String> {
    self
      .expect(TokenKind::Identifier)
      .map(|token| Expression::Variable { name: token.lexeme })
  }

  fn string(&mut self) -> Result<Expression, String> {
    self
      .expect(TokenKind::String)
      .map(|token| Expression::String {
        value: token.lexeme,
      })
  }

  pub fn expression(&mut self) -> Result<Expression, String> {
    match self.kind() {
      TokenKind::Let => self.let_expression(),
      TokenKind::Case => self.match_case_expression(),
      TokenKind::If => self.if_expression(),
      _ => self.infix(0),
    }
  }

  fn if_expression(&mut self) -> Result<Expression, String> {
    self.expect(TokenKind::If)?;
    let condition = self.expression()?;
    self.expect(TokenKind::Then)?;
    let then_branch = self.expression()?;
    self.expect(TokenKind::Else)?;
    let else_branch = self.expression()?;
    Ok(Expression::If {
      condition: Box::new(condition),
      then_branch: Box::new(then_branch),
      else_branch: Box::new(else_branch),
    })
  }

  fn infix(&mut self, prec: usize) -> Result<Expression, String> {
    if prec > PREC.len() - 1 {
      return self.call();
    }

    let mut left = self.infix(prec + 1)?;

    while PREC[prec].iter().any(|a| self.is(*a)) {
      let op = self.operation()?;
      let right = self.infix(prec + 1)?;
      left = Expression::Binary {
        op,
        lhs: Box::new(left),
        rhs: Box::new(right),
      };
    }

    Ok(left)
  }

  fn operation(&mut self) -> Result<Operation, String> {
    let token = self.eat();
    match token.kind {
      TokenKind::Plus => Ok(Operation::Add),
      TokenKind::Minus => Ok(Operation::Sub),
      TokenKind::Star => Ok(Operation::Mul),
      TokenKind::Slash => Ok(Operation::Div),
      TokenKind::DoubleEquals => Ok(Operation::Equal),
      _ => Err(format!("Expected operator, got {:?}", token.lexeme)),
    }
  }

  fn call(&mut self) -> Result<Expression, String> {
    let callee = self.primary()?;
    if self.is(TokenKind::LParens) {
      self.eat();
      let mut arguments = vec![];
      while !self.is(TokenKind::RParens) {
        arguments.push(self.expression()?);
        if self.is(TokenKind::RParens) {
          break;
        }
        self.expect(TokenKind::Comma)?;
      }
      self.expect(TokenKind::RParens)?;
      Ok(Expression::Call {
        callee: Box::new(callee),
        arguments,
      })
    } else {
      Ok(callee)
    }
  }

  fn let_expression(&mut self) -> Result<Expression, String> {
    self.expect(TokenKind::Let)?;
    let bind = self.expect(TokenKind::Identifier)?;
    self.expect(TokenKind::Equals)?;
    let value = self.expression()?;
    self.expect(TokenKind::In)?;
    let next = self.expression()?;
    Ok(Expression::Let {
      bind: bind.lexeme,
      value: Box::new(value),
      next: Box::new(next),
    })
  }

  fn match_case_expression(&mut self) -> Result<Expression, String> {
    self.expect(TokenKind::Case)?;
    let mut scrutinee = vec![self.expression()?];
    while self.is(TokenKind::Comma) {
      self.eat();
      scrutinee.push(self.expression()?);
    }
    self.expect(TokenKind::Of)?;

    let mut arms = vec![self.arm()?];
    while self.is(TokenKind::Semicolon) {
      self.eat();
      arms.push(self.arm()?);
    }
    self.expect(TokenKind::End)?;

    Ok(Expression::Match { scrutinee, arms })
  }

  fn arm(&mut self) -> Result<Arm, String> {
    let mut lhs = vec![self.pattern()?];
    while self.is(TokenKind::Comma) {
      self.eat();
      lhs.push(self.pattern()?);
    }

    self.expect(TokenKind::Arrow)?;
    let rhs = self.expression()?;
    Ok(Arm {
      lhs,
      rhs: Box::new(rhs),
    })
  }

  fn pattern(&mut self) -> Result<Pattern, String> {
    match self.kind() {
      TokenKind::Wildcard => self.expect(TokenKind::Wildcard).map(|_| Pattern::Wildcard),
      TokenKind::Atom => self.expect(TokenKind::Atom).map(|token| Pattern::Atom {
        value: token.lexeme,
      }),
      TokenKind::Number => self.expect(TokenKind::Number).map(|token| Pattern::Number {
        value: token.lexeme.parse().unwrap(),
      }),
      TokenKind::Identifier => self
        .expect(TokenKind::Identifier)
        .map(|token| Pattern::Variable { name: token.lexeme }),
      TokenKind::String => self.expect(TokenKind::String).map(|token| Pattern::String {
        value: token.lexeme,
      }),
      TokenKind::LBrace => {
        self.expect(TokenKind::LBrace)?;
        let mut elements = vec![];
        while !self.is(TokenKind::RBrace) {
          elements.push(self.pattern()?);
          if self.is(TokenKind::RBrace) {
            break;
          }
          self.expect(TokenKind::Comma)?;
        }
        self.expect(TokenKind::RBrace)?;
        Ok(Pattern::Tuple { elements })
      }
      _ => todo!(),
    }
  }

  pub fn fn_definition(&mut self) -> Result<FnDefinition, String> {
    self.expect(TokenKind::Fn)?;
    let name = self.expect(TokenKind::Identifier)?;
    let mut clauses = vec![self.fn_clause()?];
    while self.is(TokenKind::Fn)
      && self.next.kind == TokenKind::Identifier
      && self.next.lexeme == name.lexeme
    {
      self.eat();
      self.eat();
      clauses.push(self.fn_clause()?);
    }
    Ok(FnDefinition {
      name: name.lexeme,
      clauses,
    })
  }

  fn fn_clause(&mut self) -> Result<FnClause, String> {
    self.expect(TokenKind::LParens)?;
    let mut patterns = vec![];
    while !self.is(TokenKind::RParens) {
      patterns.push(self.pattern()?);
      if self.is(TokenKind::RParens) {
        break;
      }
      self.expect(TokenKind::Comma)?;
    }
    self.expect(TokenKind::RParens)?;
    self.expect(TokenKind::Arrow)?;
    let body = self.expression()?;
    Ok(FnClause {
      patterns,
      body: Box::new(body),
    })
  }

  pub fn program(&mut self) -> Result<Program, String> {
    let mut definitions = vec![];
    while !self.is(TokenKind::Eof) {
      definitions.push(self.fn_definition()?);
    }
    Ok(Program { definitions })
  }
}

#[cfg(test)]
mod test {
  use crate::lexer::Lexer;

  use super::Parser;

  #[test]
  fn parser_test() {
    let src = r#"
fn id({#batata}) -> 42
fn id(#error) -> 0
fn id(x) ->
  case x of
    {} -> 1;
    _ -> 2
  end

fn main() -> id(42)
"#;
    let mut parser = Parser::new(Lexer::new(src));
    let expression = parser.program().and_then(|p| p.desugar());
    println!("{expression:?}")
  }
}
