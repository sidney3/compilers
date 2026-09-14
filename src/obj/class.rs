use lox_derive::LoxObj;

use super::ObjData;
use crate::gc::{Trace, Tracer};
use crate::obj::BoundMethod;
use crate::obj::Closure;
use crate::obj::Obj;
use crate::runtime::RuntimeError;
use crate::runtime::Symbol;
use crate::runtime::{Runtime, Value};
use std::collections::HashMap;

#[derive(Debug, LoxObj)]
pub struct Class {
  name: Symbol,
  methods: Vec<Obj<Closure>>,
}

#[derive(Debug, LoxObj)]
pub struct Instance {
  ident: Symbol,
  properties: HashMap<Symbol, Value>,
  methods: Vec<Obj<BoundMethod>>,
}

impl Class {
  pub fn new(name: Symbol) -> Self {
    Self {
      name,
      methods: Vec::new(),
    }
  }

  pub fn add_method(&mut self, rt: &Runtime, method: Obj<Closure>) {
    let name = method.borrow(rt).borrow_func(rt).name;

    if let Some(i) = self
      .methods
      .iter()
      .position(|m| m.borrow(rt).borrow_func(rt).name == name)
    {
      self.methods.swap_remove(i);
    }

    self.methods.push(method);
  }

  pub fn load_method(&self, rt: &Runtime, method_name: Symbol) -> Option<Obj<Closure>> {
    self
      .methods()
      .iter()
      .find(|f| f.borrow(rt).borrow_func(rt).name == method_name)
      .cloned()
  }

  pub fn symbol(&self) -> Symbol {
    self.name
  }

  // NB: these functions are unsafe to call directly
  // as `this` and `super` are unbound. Instance methods
  // are NOT a copy of these methods. They are closures
  // with these attributes properly bound.
  pub fn methods(&self) -> &Vec<Obj<Closure>> {
    &self.methods
  }
}

impl Instance {
  pub fn new(name: Symbol) -> Self {
    Self {
      ident: name,
      properties: HashMap::new(),
      methods: Vec::new(),
    }
  }

  // Should only be called during construction time. This
  // can't happen in the constructor because bound methods
  // need to actually be bound to the class...
  pub fn add_method(&mut self, method: Obj<BoundMethod>) {
    self.methods.push(method);
  }

  pub fn equals(&self, rt: &Runtime, rhs: &Instance) -> std::result::Result<bool, RuntimeError> {
    if self.ident != rhs.ident {
      let msg = format!(
        "TypeError: comparison {}=={} is not defined",
        rt.resolve_sym(self.ident),
        rt.resolve_sym(rhs.ident)
      );
      Err(RuntimeError::new(msg.as_str()))
    } else {
      self
        .properties
        .iter()
        .try_fold(
          true,
          |accum, (k, v)| -> std::result::Result<bool, RuntimeError> {
            rhs
              .properties
              .get(k)
              .map(|v_| v.equals(*v_, rt))
              .unwrap_or(Ok(false))
              .map(|next| next && accum)
          },
        )
        .map(|b| b && self.properties.len() == rhs.properties.len())
    }
  }

  pub fn load_attr(&self, rt: &Runtime, sym: Symbol) -> Option<Value> {
    // TODO: this changes to be much faster once we have a vtable
    self.properties.get(&sym).cloned().or_else(|| {
      self
        .methods
        .iter()
        .find(|m| rt.closure_func(&m.borrow(rt).closure()).name == sym)
        .map(|m| m.as_value())
    })
  }
  pub fn set_attr(
    &mut self,
    rt: &Runtime,
    sym: Symbol,
    to: Value,
  ) -> std::result::Result<(), RuntimeError> {
    if self
      .methods
      .iter()
      .any(|m| rt.closure_func(&m.borrow(rt).closure()).name == sym)
    {
      return Err(RuntimeError::new(format!("Attempt to set attribute {} of class that already belongs to a method. Lox does not support rebinding of methods names",
      rt.resolve_sym(sym),
      ).as_str()));
    }
    self.properties.insert(sym, to);

    Ok(())
  }
  pub fn name<'a>(&self, rt: &'a Runtime) -> &'a str {
    rt.resolve_sym(self.ident)
  }
}

impl Trace<ObjData> for Class {
  fn trace(&self, tracer: &mut Tracer<ObjData>) {
    for method in &self.methods {
      method.trace(tracer);
    }
  }
}

impl Trace<ObjData> for Instance {
  fn trace(&self, tracer: &mut Tracer<ObjData>) {
    for member in self.properties.values() {
      member.trace(tracer);
    }
    for method in &self.methods {
      method.trace(tracer);
    }
  }
}
