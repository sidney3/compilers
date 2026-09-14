use std::hash::Hash;

use lasso::Spur;

use lox_core::{Span, Spans};

pub trait TokenType: Hash + Eq + Clone + Copy + PartialEq {
  fn eof() -> Self;
  fn whitespace() -> Self;
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Token<T: TokenType> {
  pub lexeme: Spur,
  pub token_type: T,
  pub span: Span,
}

impl<T: TokenType> Spans for Token<T> {
  fn span(&self) -> Span {
    self.span
  }
}
