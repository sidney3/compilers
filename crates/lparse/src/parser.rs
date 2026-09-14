use lasso::Rodeo;
use log::debug;
use lox_core::Span;
use lox_core::{InternPool, Ordinal};
use ndarray::Array2;
use nonempty::NonEmpty;

use super::action::{Action, make_action};
use super::debug::DisplayWithGrammarExt;
use super::error::{Error, Result};
use super::goto::make_goto;
use super::grammar::*;
use super::rule::Rule;
use super::state::{State, StateId};
use lexer::{Token, Tokens};

#[derive(Debug)]
pub struct Tree<R: Rule> {
  pub lexeme_arena: Rodeo,
  pub root: Node<R>,
}

#[derive(Debug)]
enum NodeChildren<R: Rule> {
  // TODO: write our own NonEmpty that has
  // head and tail contiguous (as we need
  // to represent the span of this).
  NonTrivial(Vec<Node<R>>),

  // When our node has >= 1 children,
  // we can infer the span.
  Trivial(usize),
}

#[derive(Debug)]
pub struct Parent<R: Rule> {
  pub rule: R,
  children: NodeChildren<R>,
}

impl<R: Rule> Parent<R> {
  pub fn make_empty(rule: R, start_position: usize) -> Self {
    Self {
      rule,
      children: NodeChildren::Trivial(start_position),
    }
  }

  pub fn children(&self) -> &[Node<R>] {
    match &self.children {
      NodeChildren::NonTrivial(children) => children.as_slice(),
      NodeChildren::Trivial(_) => &[],
    }
  }

  pub fn new(rule: R, children: NonEmpty<Node<R>>) -> Self {
    Self {
      rule,
      children: NodeChildren::NonTrivial(children.into_iter().collect()),
    }
  }
}

#[derive(Debug)]
pub enum Node<R: Rule> {
  Leaf(Token<R::TokenType>),
  Parent(Parent<R>),
}

impl<R: Rule> Node<R> {
  pub fn symbol(&self) -> Symbol<R> {
    match self {
      Self::Leaf(token) => Symbol::Token(token.token_type),
      Self::Parent(Parent {
        rule,
        children: _,
        span: _,
      }) => Symbol::Rule(*rule),
    }
  }

  pub fn span(&self) -> Span {
    match self {
      Self::Leaf(token) => token.span,
      Self::Parent(parent) => parent.span(),
    }
  }
}

pub struct Parser<R: Rule> {
  grammar: Grammar<R>,

  state_table: InternPool<StateId, State>,
  goto_table: Array2<Option<StateId>>,
  action_table: Array2<Action>,
  initial_state_id: StateId,
}

type Stack<R> = Vec<(StateId, Node<R>)>;

impl<R: Rule> Parser<R> {
  pub fn new(grammar: Grammar<R>) -> Self {
    let mut state_table = InternPool::new();

    let goto_table = make_goto(&grammar, &mut state_table);
    let action_table = make_action(&grammar, &state_table);
    let initial_state_id = state_table.get_or_intern(State::initial(&grammar));

    Self {
      grammar,
      initial_state_id,
      state_table,
      goto_table,
      action_table,
    }
  }

  pub fn parse(&self, tokens: Tokens<R::TokenType>) -> Result<Tree<R>> {
    debug!("start parse");
    let mut curr_state_id = self.initial_state_id;
    let mut stack = Stack::<R>::new();

    let mut iter = tokens.iter().peekable();

    loop {
      let next_token = iter.peek().ok_or(Error::IncompleteProgram).cloned()?;
      let focus = Span::trivial();

      debug!(
        "Current state: {}",
        self.state_table[curr_state_id].with(&self.grammar)
      );

      // We want to support rules that return nodes that are _not_ of the type of that rule.
      let action = &self.action_table[[curr_state_id.0, next_token.token_type.ord()]];

      let next_node: Node<R> = match action {
        Action::Shift => match iter.next() {
          Some(token) => {
            debug!(
              "shift token {:?} with lexeme \"{}\"",
              token.token_type,
              tokens.lexeme_arena.resolve(&token.lexeme)
            );
            Node::Leaf(*token)
          }
          None => return Err(Error::ExpectedToken(focus)),
        },
        Action::Reduce(production_id) | Action::Accept(production_id) => {
          let production = self.grammar.production(*production_id);

          if let Action::Reduce(_) = action {
            debug!("Reduce to {}", production.rule);
          }

          let drain_start = stack
            .len()
            .checked_sub(production.len())
            .expect("Attempted to drain more stack elements than are complete");
          let mut drained = stack.drain(drain_start..stack.len());

          let parent = match drained.next() {
            // Reduce to a non-empty production, our state is restored to what it
            // was when we reduced to ``head``
            Some(head) => {
              curr_state_id = head.0;
              let elements = NonEmpty {
                head: head.1,
                tail: drained.map(|(_, node)| node).collect(),
              };

              Parent::new(production.rule, elements)
            }
            // Reduce to an empty production, our state doesn't change
            None => Parent::make_empty(production.rule, focus.start),
          };

          Node::Parent(parent)
        }
      };

      if let Action::Accept(_) = action {
        if !stack.is_empty() {
          return Err(Error::ExcessProgram(next_token.span));
        }

        match next_node {
          Node::Parent(p) if p.rule == self.grammar.target_rule() => {
            return Ok(Tree {
              lexeme_arena: tokens.lexeme_arena,
              root: Node::Parent(p),
            });
          }
          _ => panic!("unreachable"),
        }
      }

      let prev_state = curr_state_id;
      match self.goto_table[[curr_state_id.0, next_node.symbol().ord()]] {
        Some(next_state_id) => {
          curr_state_id = next_state_id;
        }
        None => {
          return Err(Error::UnexpectedToken(next_token.span));
        }
      }

      stack.push((prev_state, next_node));
    }
  }
}
