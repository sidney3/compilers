use std::fmt;
use std::ops::Range;

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, PartialOrd)]
pub struct Span {
  pub start: usize,
  pub end: usize,
}

impl Span {
  pub const fn new(start: usize, end: usize) -> Self {
    Self { start, end }
  }

  pub fn lexeme<'a>(&self, program: &'a str) -> &'a str {
    &program[self.start..self.end]
  }

  pub fn trivial() -> Self {
    Span::new(0, 0)
  }

  pub fn point(start: usize) -> Self {
    Self { start, end: start }
  }

  pub fn combine_overlapping(self, other: Span) -> Option<Self> {
    let (earlier, later) = if self.start <= other.start {
      (self, other)
    } else {
      (other, self)
    };

    if earlier.end < later.start {
      None
    } else {
      Some(Span::new(earlier.start, later.end))
    }
  }

  pub fn range(&self) -> Range<usize> {
    Range {
      start: self.start,
      end: self.end,
    }
  }
}

impl fmt::Display for Span {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "[{},{})", self.start, self.end)
  }
}

/// A trait describing some type spanning a set of bytes
/// (the space of bytes that get spanned over should be
/// inferrable from the context).
///
/// The derived default implementations for the trait make
/// the assumption that your struct members are layed out
/// in the order that they appear in the file, and that
/// all textual objects from the code appear in the struct
///
/// For example, we would want to include _class, _lbrace
/// and _rbraace in our ClassDeclaration type so that
/// we can properly figure out where this type lives.
///
/// ```
/// struct Token {};
/// struct ClassDeclaration {
///   _class: Token,
///   name: Token,
///   _lbrace: Token,
///   //...
///   _rbrace: Token,
/// };
/// ```
///
pub trait Spans {
  fn span(&self) -> Span;
}
