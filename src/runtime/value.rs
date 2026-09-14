use std::ops::Deref;

use super::{Handle, Runtime, RuntimeError};
use crate::gc::{Trace, Tracer};
use crate::obj::{ObjData, UpValue};
use log::info;

pub type Num = f64;
pub type Bool = bool;

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
pub enum Value {
  Num(Num),
  Obj(Handle),
  Bool(Bool),
  Nil,
}

impl Trace<ObjData> for Value {
  fn trace(&self, tracer: &mut Tracer<ObjData>) {
    if let &Value::Obj(handle) = self {
      tracer.mark(handle);
    }
  }
}

impl TryFrom<&Value> for Bool {
  type Error = RuntimeError;

  fn try_from(value: &Value) -> Result<Self, Self::Error> {
    match value {
      &Value::Bool(b) => Ok(b),
      &Value::Num(x) => Ok(x != 0f64),
      Value::Obj(_) => Err(RuntimeError::new("Obj cannot be converted to bool")),
      Value::Nil => Ok(false),
    }
  }
}

// Value::add might produce a new `Object` and thus require a heap allocation.
// However, we'd really like `add` to be a pure function... Which this type
// allows us to model
pub enum ProtoValue {
  Imm(Value),
  Obj(ObjData), // to be put on the heap
}

impl ProtoValue {
  pub fn into_value(self, vm: &mut Runtime) -> Value {
    match self {
      Self::Imm(val) => val,
      Self::Obj(obj) => Value::Obj(vm.alloc(obj)),
    }
  }
}

impl Value {
  //
  fn flatten(self, vm: &Runtime) -> Self {
    match self {
      Self::Obj(obj) => match vm.borrow(obj).deref() {
        ObjData::UpValue(UpValue::Closed(inner)) => inner.flatten(vm),
        ObjData::UpValue(UpValue::Open { absolute_stack_pos }) => {
          vm.value_stack[*absolute_stack_pos].flatten(vm)
        }
        _ => self,
      },
      _ => self,
    }
  }

  pub fn add(self, rhs: Value, vm: &Runtime) -> Result<ProtoValue, RuntimeError> {
    match (self.flatten(vm), rhs.flatten(vm)) {
      (Value::Num(a), Value::Num(b)) => Ok(ProtoValue::Imm(Value::Num(a + b))),
      (Value::Obj(a), Value::Obj(b)) => {
        let lhs = vm.borrow(a);
        let rhs = vm.borrow(b);
        lhs.add(&rhs, vm)
      }

      _ => {
        if let Value::Obj(obj) = self {
          info!("LoxVal: {}", vm.borrow(obj).deref())
        }

        Err(RuntimeError {
          msg: format!("Bad binary expression between: {:?}, {:?}", self, rhs,),
        })
      }
    }
  }

  pub fn divide(self, rhs: Value, vm: &Runtime) -> Result<Value, RuntimeError> {
    match (self.flatten(vm), rhs.flatten(vm)) {
      (Value::Num(a), Value::Num(b)) => {
        if b != 0f64 {
          Ok(Value::Num(a / b))
        } else {
          Err(RuntimeError {
            msg: "Divide by zero".to_string(),
          })
        }
      }
      _ => Err(RuntimeError {
        msg: format!("Bad divide between: {:?}, {:?}", self, rhs),
      }),
    }
  }

  pub fn equals(self, rhs: Value, vm: &Runtime) -> Result<bool, RuntimeError> {
    let eq = match (self.flatten(vm), rhs.flatten(vm)) {
      (Value::Num(a), Value::Num(b)) => a == b,
      (Value::Obj(a), Value::Obj(b)) => vm.borrow(a).equals(&vm.borrow(b), vm)?,
      (Value::Bool(a), Value::Bool(b)) => a == b,
      (Value::Nil, Value::Nil) => true,
      _ => {
        return Err(RuntimeError {
          msg: "Incompatible binary operators to ==".to_string(),
        });
      }
    };
    Ok(eq)
  }
  pub fn equals_value(self, rhs: Value, vm: &Runtime) -> Result<Value, RuntimeError> {
    let x = self.equals(rhs, vm)?;
    Ok(Value::Bool(x))
  }
  pub fn neq(self, rhs: Value, vm: &Runtime) -> Result<Value, RuntimeError> {
    let x = self.equals(rhs, vm)?;

    Ok(Value::Bool(!x))
  }
  pub fn mult(self, rhs: Value, vm: &Runtime) -> Result<Value, RuntimeError> {
    self.binary_arithmetic(rhs, |a, b| Value::Num(a * b), "*", vm)
  }

  pub fn sub(self, rhs: Value, vm: &Runtime) -> Result<Value, RuntimeError> {
    self.binary_arithmetic(rhs, |a, b| Value::Num(a - b), "-", vm)
  }

  pub fn greater(self, rhs: Value, vm: &Runtime) -> Result<Value, RuntimeError> {
    self.binary_arithmetic(rhs, |a, b| Value::Bool(a > b), ">", vm)
  }

  pub fn less(self, rhs: Value, vm: &Runtime) -> Result<Value, RuntimeError> {
    self.binary_arithmetic(rhs, |a, b| Value::Bool(a < b), "<", vm)
  }
  pub fn leq(self, rhs: Value, vm: &Runtime) -> Result<Value, RuntimeError> {
    self.binary_arithmetic(rhs, |a, b| Value::Bool(a <= b), "<=", vm)
  }
  pub fn geq(self, rhs: Value, vm: &Runtime) -> Result<Value, RuntimeError> {
    self.binary_arithmetic(rhs, |a, b| Value::Bool(a >= b), ">=", vm)
  }

  pub fn not(self, vm: &Runtime) -> Result<Value, RuntimeError> {
    match self.flatten(vm) {
      Value::Bool(b) => Ok(Value::Bool(!b)),
      _ => Err(RuntimeError::new("Invalid input to not")),
    }
  }

  pub fn unary_minus(self, vm: &Runtime) -> Result<Value, RuntimeError> {
    match self.flatten(vm) {
      Value::Num(x) => Ok(Value::Num(-x)),
      _ => Err(RuntimeError::new("Invalid input to minus")),
    }
  }

  pub fn repr(self, vm: &Runtime) -> String {
    match self.flatten(vm) {
      Value::Num(x) => x.to_string(),
      Value::Obj(o) => {
        format!("{}", *vm.borrow(o))
      }
      Value::Bool(b) => if b { "True" } else { "False" }.to_string(),
      Value::Nil => "nil".to_string(),
    }
  }

  fn binary_arithmetic<F: FnOnce(Num, Num) -> Value>(
    self,
    rhs: Value,
    op: F,
    op_name: &str,
    vm: &Runtime,
  ) -> Result<Value, RuntimeError> {
    match (self.flatten(vm), rhs.flatten(vm)) {
      (Value::Num(a), Value::Num(b)) => Ok(op(a, b)),
      _ => Err(RuntimeError {
        msg: format!(
          "Incompatible binary expression: {:?} {op_name} {:?}",
          self, rhs
        ),
      }),
    }
  }
}
