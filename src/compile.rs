use crate::asm::{FuncStack, FuncState, InstructionKind, Label};
use crate::frontend::ast::{
  Assign, Binary, Block, ClassDeclaration, ElseTail, ForLoop, IfStatement, LValue, Member, Unary,
  VarDeclaration, WhileStatement,
};
use crate::frontend::ast::{Ast, BinOp, Declaration, Expression, Literal, Statement, UnaryOp};
use crate::frontend::token::Ident;
use crate::obj::{Function, LoxString, ObjData};
use crate::runtime::{Root, Runtime, Symbol, Value};

pub struct Compiler<'a, 'vm> {
  compile_stack: FuncStack,
  ast: &'a Ast,
  rt: &'vm mut Runtime,
}

impl<'a, 'vm> Compiler<'a, 'vm> {
  pub fn new(ast: &'a Ast, rt: &'vm mut Runtime) -> Self {
    let main_sym = rt.get_or_intern_sym_str("main");
    Self {
      compile_stack: FuncStack::new(FuncState::new(rt, main_sym, 0, None)),
      ast,
      rt,
    }
  }

  pub fn compile(mut self) -> Root<Function> {
    self.ast(self.ast);

    self
      .compile_stack
      .pop_last(self.rt)
      .expect("Finished compilation with excess remaining frames")
  }

  pub fn load_ident_sym(&mut self, ident: Ident) -> Symbol {
    self.load_str_sym(self.ident_sym(ident))
  }

  pub fn load_str_sym(&mut self, s: &str) -> Symbol {
    self.rt.get_or_intern_sym_str(s)
  }

  fn ident_sym(&self, ident: Ident) -> &'a str {
    self.ast.lexeme_arena.resolve(&ident)
  }

  // Accessors to the current compiling func
  fn func_mut(&mut self) -> &mut FuncState {
    self.compile_stack.head_mut()
  }

  fn ast(&mut self, ast: &Ast) {
    for decl in &ast.root.declarations {
      self.decl(decl);
    }
    self.func_mut().ret();
  }

  fn constant(&mut self, constant: Value) {
    self.compile_stack.head_mut().constant(self.rt, constant);
  }

  fn function(
    &mut self,
    name: Symbol,
    args: &[Symbol],
    body: impl FnOnce(&mut Self),
  ) -> Root<Function> {
    // See docs/calling_convention.md
    self.compile_stack.push(self.rt, name, args.len());
    self.func_mut().add_local(name);

    for arg in args {
      self.func_mut().add_local(*arg);
    }

    body(self);

    self.constant(Value::Nil);
    self.func_mut().ret();

    self
      .compile_stack
      .pop(self.rt)
      .expect("Compilation stack too small")
  }
  fn trivial_function(&mut self, name: Symbol) -> Root<Function> {
    self.function(name, &[], |_| ())
  }
  fn closure(&mut self, func: Root<Function>) {
    let name = func.as_obj().borrow(self.rt).name;
    self
      .compile_stack
      .head_mut()
      .add_closure_to_scope(self.rt, func.as_obj(), name);
    func.free(self.rt);
  }

  fn load_symbols(&mut self, idents: &[Ident]) -> Vec<Symbol> {
    idents
      .iter()
      .cloned()
      .map(|i| self.load_ident_sym(i))
      .collect()
  }

  fn var_decl(&mut self, var_decl: &VarDeclaration) {
    self.expr(&var_decl.assign);
    let sym = self.load_ident_sym(var_decl.ident);
    self.func_mut().define_var(sym);
  }
  fn decl(&mut self, decl: &Declaration) {
    match decl {
      Declaration::Statement(s) => self.statement(s),
      Declaration::Var(var) => {
        self.var_decl(var);
      }
      Declaration::Class(class) => {
        self.make_class(class);
      }
      Declaration::Fun(f) => {
        let f_name = self.load_ident_sym(f.name);
        let args = self.load_symbols(f.args.as_slice());
        let func = self.function(f_name, args.as_slice(), |this| this.block(&f.body));
        self.closure(func);
      }
    };
  }

  fn make_class(&mut self, class: &ClassDeclaration) {
    let name = self.load_ident_sym(class.ident);
    self.func_mut().make_class(name);
    self.func_mut().define_var(name);

    if let Some(superclass) = class.inherits.map(|i| self.load_ident_sym(i)) {
      // Invoke InstructionKind::Inherit and bind super
      self.func_mut().begin_scope(); // Begin super scope

      // This superclass gets used as an argument to OP_INHERIT,
      // which leaves it intact on the stack, AND to bind `super`.
      // The runtime does not access this superclass directly.
      // Instead, the `super.<method>` keyword gets compiled
      // into a special instruction LoadSuperMethod.
      self.func_mut().load_var(superclass);

      let super_sym = self.load_str_sym("super");
      self.func_mut().add_local(super_sym);
      self.func_mut().load_var(name);
      self.func_mut().inherit();
    }

    self.func_mut().load_var(name);

    for method in &class.methods {
      let name = self.load_ident_sym(method.name);
      let args = self.load_symbols(&method.args);

      let method = self.function(name, &args, |this| this.method_body(&method.body));

      self
        .compile_stack
        .head_mut()
        .add_method(self.rt, method.as_obj());
      method.free(self.rt);
    }

    // For simplicity, every class gets a constructor. A wiser man might
    // choose to strongly type this.
    let init_sym = self.rt.init_sym();
    if !class
      .methods
      .iter()
      .any(|m| self.load_ident_sym(m.name) == init_sym)
    {
      let ctor = self.trivial_function(init_sym);
      self
        .compile_stack
        .head_mut()
        .add_method(self.rt, ctor.as_obj());
      ctor.free(self.rt);
    }

    self.func_mut().pop(); // pop this

    if class.inherits.is_some() {
      self.func_mut().end_scope(); // End super scope
    }
  }

  fn method_body(&mut self, body: &Block) {
    let this = self.load_str_sym("this");
    self.func_mut().bind_this(this);
    self.block(body);
  }

  fn statement(&mut self, statement: &Statement) {
    match statement {
      Statement::Expr(e) => {
        self.expr(&e.expr);
        self.func_mut().pop();
      }
      Statement::Block(block) => {
        self.block(block);
      }
      Statement::If(if_statement) => {
        let after_if = self.func_mut().create_label("after if");
        self.if_stmnt(if_statement, after_if);
        self.func_mut().bind_label(after_if);
      }

      Statement::While(WhileStatement { cond, body }) => {
        self.while_statement(cond, |this| this.block(body));
      }
      Statement::Break => self.func_mut().loop_break(),
      Statement::Return(ret) => {
        // See docs/calling_convention.md
        self.expr(&ret.expr);
        self.func_mut().ret();
      }
      Statement::For(ForLoop {
        init,
        increment,
        condition,
        body,
      }) => {
        self.func_mut().begin_scope();

        self.var_decl(init);

        self.while_statement(condition, |this| {
          this.expr(increment);
          this.func_mut().pop(); //< a little bit annoying that we have to write this
          this.block(body)
        });

        self.func_mut().end_scope();
      }
    };
  }

  fn while_statement(&mut self, cond: &Expression, body: impl FnOnce(&mut Self)) {
    let while_start = self.func_mut().create_label("while start");
    let after_while = self.func_mut().create_label("after while");
    self.func_mut().begin_loop(after_while);
    self.func_mut().bind_label(while_start);

    self.expr(cond);
    self.func_mut().jmp_if_false(after_while);
    body(self);
    self.func_mut().jmp(while_start);
    self.func_mut().bind_label(after_while);
    self.func_mut().end_loop();
  }
  fn if_stmnt(&mut self, if_stmnt: &IfStatement, end_label: Label) {
    match if_stmnt {
      IfStatement::Trivial { cond, body } => {
        self.expr(cond);
        self.func_mut().jmp_if_false(end_label);
        self.block(body);
      }
      IfStatement::Fork {
        cond,
        true_case,
        false_case,
      } => {
        let after_fst_branch = self.func_mut().create_label("after first branch");
        self.expr(cond);
        self.func_mut().jmp_if_false(after_fst_branch);

        self.block(true_case);
        self.func_mut().jmp(end_label);
        self.func_mut().bind_label(after_fst_branch);

        match false_case {
          ElseTail::Trivial(tail) => self.block(tail),
          ElseTail::If(recurse) => self.if_stmnt(recurse, end_label),
        }
      }
    }
  }

  fn block(&mut self, block: &Block) {
    self.func_mut().begin_scope();

    for decl in &block.declarations {
      self.decl(decl);
    }

    self.func_mut().end_scope();
  }

  fn and(&mut self, lhs: &Expression, rhs: &Expression) {
    // short circuit: if we are false, return immediately. Otherwise, execute and return rhs
    let after_and = self.func_mut().create_label("after and");
    self.expr(lhs);

    self.func_mut().jmp_if_false_preserving(after_and);

    self.expr(rhs);
    self.func_mut().bind_label(after_and);
  }

  fn or(&mut self, lhs: &Expression, rhs: &Expression) {
    // short circuit: if we are true, return immediately. Otherwise, execute and return rhs
    let after_or = self.func_mut().create_label("after or");
    self.expr(lhs);
    self.func_mut().jmp_if_true_preserving(after_or);

    self.expr(rhs);
    self.func_mut().bind_label(after_or);
  }

  fn expr(&mut self, expr: &Expression) {
    match expr {
      Expression::Nil => {
        self.constant(Value::Nil);
      }
      Expression::Bin(Binary { op, lhs, rhs }) => {
        let instruction_kind = match op {
          BinOp::And => return self.and(lhs.as_ref(), rhs.as_ref()),
          BinOp::Or => return self.or(lhs.as_ref(), rhs.as_ref()),

          BinOp::Times => InstructionKind::Mult,
          BinOp::Minus => InstructionKind::Sub,
          BinOp::Divide => InstructionKind::Divide,
          BinOp::Plus => InstructionKind::Add,
          BinOp::Geq => InstructionKind::Geq,
          BinOp::Leq => InstructionKind::Leq,
          BinOp::Equals => InstructionKind::Equals,
          BinOp::Less => InstructionKind::Less,
          BinOp::Greater => InstructionKind::Greater,
          BinOp::Neq => InstructionKind::Neq,
        };
        self.expr(lhs.as_ref());
        self.expr(rhs.as_ref());

        // TODO: remove and make emit_nullary private
        self.func_mut().emit_nullary(instruction_kind);
      }
      Expression::Unary(Unary { op, operand }) => {
        self.expr(operand.as_ref());
        let instruction_kind = match op {
          UnaryOp::Not => InstructionKind::Not,
          UnaryOp::Minus => InstructionKind::UnaryMinus,
        };
        // TODO: remove and make emit_nullary private
        self.func_mut().emit_nullary(instruction_kind);
      }
      Expression::Lit(literal) => {
        self.lit(literal);
      }
      Expression::Assign(Assign { assignee, assign }) => {
        self.expr(assign);
        match assignee {
          &LValue::Var(v) => {
            let sym = self.load_ident_sym(v);
            self.func_mut().set_variable(sym);
          }
          LValue::Member(Member { accessee, property }) => {
            self.expr(accessee); //< this had better produce a class!
            let property_name = self.load_ident_sym(*property);
            self.compile_stack.head_mut().set_class_attr(property_name);
          }
        }
      }
      Expression::Call(call) => {
        // See docs/calling_convention.md
        self.expr(&call.callee);
        for arg in &call.args {
          self.expr(arg);
        }

        self.func_mut().callq(call.args.len());
      }

      Expression::Member(Member { accessee, property }) => {
        self.expr(accessee); //< this had better produce a class
        let property_name = self.load_ident_sym(*property);
        self.compile_stack.head_mut().get_class_attr(property_name);
      }
      &Expression::SuperMember(member) => {
        let property_name = self.load_ident_sym(member);
        let this_sym = self.load_str_sym("this");
        let super_sym = self.load_str_sym("super");

        self
          .func_mut()
          .load_super_method(this_sym, super_sym, property_name);
      }
    }
  }

  fn lit(&mut self, lit: &Literal) {
    match lit {
      Literal::Num(x) => self.constant(Value::Num(
        self
          .ast
          .lexeme_arena
          .resolve(x)
          .parse()
          .expect("Int token incorrectly defined"),
      )),
      Literal::String(string_ident) => {
        let x = self
          .ast
          .lexeme_arena
          .resolve(string_ident)
          .strip_prefix("\"")
          .and_then(|s| s.strip_suffix("\""))
          .unwrap()
          .to_string();
        let string_val = Value::Obj(self.rt.alloc(ObjData::LoxString(LoxString::new(x.clone()))));
        self.constant(string_val);
      }
      &Literal::Bool(x) => self.constant(Value::Bool(x)),
      Literal::Var(v) => {
        let sym = self.load_ident_sym(*v);
        self.func_mut().load_var(sym);
      }
    };
  }
}
