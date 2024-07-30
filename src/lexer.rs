use std::{iter::Peekable, str::Chars};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenKind {
  Identifier,
  Wildcard,
  Atom,
  Number,
  String,
  LParens,
  RParens,
  LBrace,
  RBrace,
  Plus,
  Minus,
  Slash,
  Star,
  Equals,
  DoubleEquals,
  Fn,
  Let,
  In,
  Case,
  Of,
  End,
  Comma,
  Semicolon,
  Period,
  Arrow,
  Error,
  Eof,
}

pub struct Token {
  pub kind: TokenKind,
  pub lexeme: String,
}

impl std::fmt::Debug for Token {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{:?} : {:?}", self.kind, self.lexeme)
  }
}

pub struct Lexer<'input> {
  peekable: Peekable<Chars<'input>>,
  src: &'input str,
  start: usize,
  index: usize,
}

impl<'input> Lexer<'input> {
  pub fn new(src: &'input str) -> Self {
    Self {
      peekable: src.chars().peekable(),
      src,
      start: 0,
      index: 0,
    }
  }

  pub fn advance(&mut self) -> Option<char> {
    let char = self.peekable.next()?;
    self.index += char.len_utf8();
    Some(char)
  }

  fn advance_while(&mut self, condition: impl Fn(&char) -> bool) {
    while let Some(char) = self.peekable.peek() {
      if condition(char) {
        self.advance();
      } else {
        break;
      }
    }
  }

  pub fn save(&mut self) {
    self.start = self.index;
  }

  pub fn qualify(&mut self) -> TokenKind {
    match &self.src[self.start..self.index] {
      "fn" => TokenKind::Fn,
      "let" => TokenKind::Let,
      "in" => TokenKind::In,
      "case" => TokenKind::Case,
      "of" => TokenKind::Of,
      "end" => TokenKind::End,
      _ => TokenKind::Identifier,
    }
  }

  fn whitespaces(&mut self) {
    self.save();
    self.advance_while(|c| c.is_ascii_whitespace());
  }

  fn token(&mut self) -> (TokenKind, String) {
    let kind = if let Some(char) = self.advance() {
      match char {
        '(' => TokenKind::LParens,
        ')' => TokenKind::RParens,
        '{' => TokenKind::LBrace,
        '}' => TokenKind::RBrace,
        '+' => TokenKind::Plus,
        '-' => {
          if let Some(char) = self.peekable.peek() {
            if char.is_ascii_digit() {
              self.advance();
              self.advance_while(|c| c.is_ascii_digit());
              TokenKind::Number
            } else if *char == '>' {
              self.advance();
              TokenKind::Arrow
            } else {
              TokenKind::Minus
            }
          } else {
            TokenKind::Minus
          }
        }
        '/' => TokenKind::Slash,
        '*' => TokenKind::Star,
        '=' => {
          if let Some('=') = self.peekable.peek() {
            self.advance();
            TokenKind::DoubleEquals
          } else {
            TokenKind::Equals
          }
        }
        ',' => TokenKind::Comma,
        ';' => TokenKind::Semicolon,
        '.' => TokenKind::Period,
        '\"' => {
          let mut s = String::new();
          while let Some(char) = self.peekable.peek() {
            if *char == '\"' {
              break;
            } else {
              s.push(self.advance().unwrap());
            }
          }
          if let Some('\"') = self.advance() {
            return (TokenKind::String, s);
          } else {
            return (TokenKind::Error, s);
          }
        }
        '#' => {
          self.save();
          self.advance_while(|c| c.is_ascii_alphanumeric());
          TokenKind::Atom
        }
        '_' => TokenKind::Wildcard,
        c if c.is_ascii_digit() => {
          self.advance_while(|c| c.is_ascii_digit());
          TokenKind::Number
        }
        c if c.is_ascii_alphabetic() => {
          self.advance_while(|c| c.is_ascii_alphanumeric());
          self.qualify()
        }
        _ => TokenKind::Error,
      }
    } else {
      TokenKind::Eof
    };
    (kind, self.src[self.start..self.index].to_owned())
  }

  pub fn next_token(&mut self) -> Token {
    self.whitespaces();
    self.save();
    let (kind, lexeme) = self.token();
    Token { kind, lexeme }
  }
}

impl<'input> Iterator for Lexer<'input> {
  type Item = Token;

  fn next(&mut self) -> Option<Self::Item> {
    if self.index >= self.src.len() {
      None
    } else {
      Some(self.next_token())
    }
  }
}

#[cfg(test)]
mod test {
  use super::Lexer;

  #[test]
  fn test_lexer() {
    let src = r#"
oi
42
#teste
-33330
"texi"
fn let x = 2 in x
#
->
match case 0
+-/*
"#;
    let lexer = Lexer::new(src);
    for token in lexer.into_iter() {
      println!("{token:?}");
    }
  }
}
