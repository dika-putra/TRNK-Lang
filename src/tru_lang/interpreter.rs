//! TRU Language Tree-Walk Interpreter
//! Evaluates an AST directly — no bytecode, no compilation step needed.

use std::collections::HashMap;
use std::cell::RefCell;
use std::rc::Rc;
use crate::tru_lang::ast::*;

// ─── Value ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),
    Nil,
    Array(Rc<RefCell<Vec<Value>>>),
    Tuple(Vec<Value>),
    Map(Rc<RefCell<HashMap<String, Value>>>),
    Struct { name: String, fields: Rc<RefCell<HashMap<String, Value>>> },
    EnumVariant { enum_name: String, variant: String, payload: Vec<Value> },
    Fn { params: Vec<Param>, body: Vec<Stmt>, env: Env },
    NativeFn(String),
    Return(Box<Value>),
    Break,
    Continue,
    Unit,
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Int(n)     => write!(f, "{}", n),
            Value::Float(n)   => write!(f, "{}", n),
            Value::Bool(b)    => write!(f, "{}", b),
            Value::Str(s)     => write!(f, "{}", s),
            Value::Nil        => write!(f, "nil"),
            Value::Unit       => write!(f, "()"),
            Value::Array(a)   => {
                let items: Vec<String> = a.borrow().iter().map(|v| format!("{}", v)).collect();
                write!(f, "[{}]", items.join(", "))
            }
            Value::Tuple(t)   => {
                let items: Vec<String> = t.iter().map(|v| format!("{}", v)).collect();
                write!(f, "({})", items.join(", "))
            }
            Value::Map(m)     => {
                let items: Vec<String> = m.borrow().iter().map(|(k, v)| format!("{}: {}", k, v)).collect();
                write!(f, "{{{}}}", items.join(", "))
            }
            Value::Struct { name, fields } => {
                let items: Vec<String> = fields.borrow().iter().map(|(k, v)| format!("{}: {}", k, v)).collect();
                write!(f, "{} {{ {} }}", name, items.join(", "))
            }
            Value::EnumVariant { variant, payload, .. } => {
                if payload.is_empty() {
                    write!(f, "{}", variant)
                } else {
                    let items: Vec<String> = payload.iter().map(|v| format!("{}", v)).collect();
                    write!(f, "{}({})", variant, items.join(", "))
                }
            }
            Value::Fn { .. }       => write!(f, "<fn>"),
            Value::NativeFn(name)  => write!(f, "<native:{}>", name),
            Value::Return(v)       => write!(f, "{}", v),
            Value::Break           => write!(f, "<break>"),
            Value::Continue        => write!(f, "<continue>"),
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Int(a),   Value::Int(b))   => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::Int(a),   Value::Float(b)) => (*a as f64) == *b,
            (Value::Float(a), Value::Int(b))   => *a == (*b as f64),
            (Value::Bool(a),  Value::Bool(b))  => a == b,
            (Value::Str(a),   Value::Str(b))   => a == b,
            (Value::Nil,      Value::Nil)       => true,
            _ => false,
        }
    }
}

// ─── Environment (scope chain) ────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Env(Rc<RefCell<EnvInner>>);

#[derive(Debug)]
struct EnvInner {
    vars: HashMap<String, Value>,
    parent: Option<Env>,
}

impl Env {
    pub fn new() -> Self {
        Env(Rc::new(RefCell::new(EnvInner {
            vars: HashMap::new(),
            parent: None,
        })))
    }

    pub fn child(parent: &Env) -> Self {
        Env(Rc::new(RefCell::new(EnvInner {
            vars: HashMap::new(),
            parent: Some(parent.clone()),
        })))
    }

    pub fn define(&self, name: &str, val: Value) {
        self.0.borrow_mut().vars.insert(name.to_string(), val);
    }

    pub fn assign(&self, name: &str, val: Value) -> bool {
        let mut inner = self.0.borrow_mut();
        if inner.vars.contains_key(name) {
            inner.vars.insert(name.to_string(), val);
            true
        } else if let Some(ref parent) = inner.parent.clone() {
            parent.assign(name, val)
        } else {
            false
        }
    }

    pub fn get(&self, name: &str) -> Option<Value> {
        let inner = self.0.borrow();
        if let Some(v) = inner.vars.get(name) {
            Some(v.clone())
        } else if let Some(ref parent) = inner.parent {
            parent.get(name)
        } else {
            None
        }
    }
}

// ─── Runtime Error ────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct RuntimeError {
    pub msg: String,
    pub line: usize,
}

impl RuntimeError {
    fn new(msg: impl Into<String>) -> Self {
        Self { msg: msg.into(), line: 0 }
    }
    fn at(msg: impl Into<String>, line: usize) -> Self {
        Self { msg: msg.into(), line }
    }
}

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[RuntimeError] {} (line {})", self.msg, self.line)
    }
}

type IResult = Result<Value, RuntimeError>;

// ─── Interpreter ─────────────────────────────────────────────────────────────

pub struct Interpreter {
    pub output: Vec<String>,
}

impl Interpreter {
    pub fn new() -> Self {
        Self { output: Vec::new() }
    }

    pub fn run(&mut self, program: &Program) -> IResult {
        let env = Env::new();
        self.load_stdlib(&env);
        crate::tru_lang::stdlib::load(&env); // extended: PI, E, math::*, io::*, etc.

        // First pass: register all top-level fn declarations
        for stmt in &program.stmts {
            if let Stmt::Fn(f) = stmt {
                env.define(&f.name, Value::Fn {
                    params: f.params.clone(),
                    body: f.body.clone(),
                    env: env.clone(),
                });
            }
        }

        let mut last = Value::Unit;
        for stmt in &program.stmts {
            last = self.exec_stmt(stmt, &env)?;
            if let Value::Return(v) = last {
                return Ok(*v);
            }
        }
        Ok(last)
    }

    pub fn load_stdlib(&self, env: &Env) {
        for name in &["println", "print", "len", "push", "pop",
                      "to_str", "to_int", "to_float", "assert", "assert_eq",
                      "range", "type_of", "exit"] {
            env.define(name, Value::NativeFn(name.to_string()));
        }
    }

    // ── statements ───────────────────────────────────────────────────────────

    fn exec_stmt(&mut self, stmt: &Stmt, env: &Env) -> IResult {
        match stmt {
            Stmt::Let(l)      => self.exec_let(l, env),
            Stmt::Fn(f)       => {
                env.define(&f.name, Value::Fn {
                    params: f.params.clone(),
                    body: f.body.clone(),
                    env: env.clone(),
                });
                Ok(Value::Unit)
            }
            Stmt::Return(e, s) => {
                let v = if let Some(e) = e { self.eval_expr(e, env)? } else { Value::Nil };
                Ok(Value::Return(Box::new(v)))
            }
            Stmt::Expr(e)     => self.eval_expr(e, env),
            Stmt::If(i)       => self.exec_if(i, env),
            Stmt::While(w)    => self.exec_while(w, env),
            Stmt::For(f)      => self.exec_for(f, env),
            Stmt::Block(stmts, _) => self.exec_block(stmts, env),
            Stmt::Break(_)    => Ok(Value::Break),
            Stmt::Continue(_) => Ok(Value::Continue),
            // Declarations that don't produce runtime values
            Stmt::Struct(s)   => { env.define(&s.name, Value::NativeFn(format!("struct:{}", s.name))); Ok(Value::Unit) }
            Stmt::Enum(e)     => { env.define(&e.name, Value::NativeFn(format!("enum:{}", e.name))); Ok(Value::Unit) }
            Stmt::Impl(_)     => Ok(Value::Unit),
            Stmt::Trait(_)    => Ok(Value::Unit),
            Stmt::Use(_, _)   => Ok(Value::Unit),
            Stmt::Mod(_, stmts, _) => {
                let child = Env::child(env);
                self.exec_block(stmts, &child)?;
                Ok(Value::Unit)
            }
        }
    }

    fn exec_let(&mut self, l: &LetStmt, env: &Env) -> IResult {
        let val = if let Some(e) = &l.value { self.eval_expr(e, env)? } else { Value::Nil };
        env.define(&l.name, val);
        Ok(Value::Unit)
    }

    fn exec_if(&mut self, i: &IfStmt, env: &Env) -> IResult {
        let cond = self.eval_expr(&i.cond, env)?;
        if self.is_truthy(&cond) {
            let child = Env::child(env);
            self.exec_block(&i.then_block, &child)
        } else if let Some(else_b) = &i.else_block {
            let child = Env::child(env);
            self.exec_block(else_b, &child)
        } else {
            Ok(Value::Unit)
        }
    }

    fn exec_while(&mut self, w: &WhileStmt, env: &Env) -> IResult {
        loop {
            let cond = self.eval_expr(&w.cond, env)?;
            if !self.is_truthy(&cond) { break; }
            let child = Env::child(env);
            match self.exec_block(&w.body, &child)? {
                Value::Break    => break,
                Value::Continue => continue,
                Value::Return(v) => return Ok(Value::Return(v)),
                _ => {}
            }
        }
        Ok(Value::Unit)
    }

    fn exec_for(&mut self, f: &ForStmt, env: &Env) -> IResult {
        let iter_val = self.eval_expr(&f.iter, env)?;
        let items = self.collect_iterable(iter_val)?;
        for item in items {
            let child = Env::child(env);
            child.define(&f.var, item);
            match self.exec_block(&f.body, &child)? {
                Value::Break    => break,
                Value::Continue => continue,
                Value::Return(v) => return Ok(Value::Return(v)),
                _ => {}
            }
        }
        Ok(Value::Unit)
    }

    pub fn exec_block(&mut self, stmts: &[Stmt], env: &Env) -> IResult {
        let mut last = Value::Unit;
        for s in stmts {
            last = self.exec_stmt(s, env)?;
            match &last {
                Value::Return(_) | Value::Break | Value::Continue => return Ok(last),
                _ => {}
            }
        }
        Ok(last)
    }

    // ── expressions ──────────────────────────────────────────────────────────

    fn eval_expr(&mut self, expr: &Expr, env: &Env) -> IResult {
        match expr {
            Expr::Int(n, _)   => Ok(Value::Int(*n)),
            Expr::Float(f, _) => Ok(Value::Float(*f)),
            Expr::Str(s, _)   => Ok(Value::Str(s.clone())),
            Expr::Bool(b, _)  => Ok(Value::Bool(*b)),
            Expr::Nil(_)      => Ok(Value::Nil),

            Expr::Ident(name, s) => {
                env.get(name).ok_or_else(|| RuntimeError::at(
                    format!("undefined variable '{}'", name), s.line,
                ))
            }

            Expr::Array(items, _) => {
                let vals: Result<Vec<_>, _> = items.iter().map(|e| self.eval_expr(e, env)).collect();
                Ok(Value::Array(Rc::new(RefCell::new(vals?))))
            }

            Expr::Tuple(items, _) => {
                let vals: Result<Vec<_>, _> = items.iter().map(|e| self.eval_expr(e, env)).collect();
                Ok(Value::Tuple(vals?))
            }

            Expr::Binary(lhs, op, rhs, s) => self.eval_binary(lhs, op, rhs, s.line, env),

            Expr::Unary(op, e, _) => {
                let v = self.eval_expr(e, env)?;
                match op {
                    UnOp::Neg   => self.neg(v),
                    UnOp::Not   => Ok(Value::Bool(!self.is_truthy(&v))),
                    UnOp::Ref   => Ok(v), // simplified — no real references
                    UnOp::Deref => Ok(v),
                }
            }

            Expr::Assign(lhs, op, rhs, s) => self.eval_assign(lhs, op, rhs, s.line, env),

            Expr::Call(callee, args, s) => self.eval_call(callee, args, s.line, env),

            Expr::MethodCall(obj, method, args, s) => self.eval_method(obj, method, args, s.line, env),

            Expr::Field(obj, field, s) => {
                let v = self.eval_expr(obj, env)?;
                match v {
                    Value::Struct { fields, .. } => {
                        fields.borrow().get(field).cloned().ok_or_else(|| RuntimeError::at(
                            format!("no field '{}'", field), s.line,
                        ))
                    }
                    _ => Err(RuntimeError::at(format!("field access on non-struct"), s.line)),
                }
            }

            Expr::Index(arr, idx, s) => {
                let arr_v = self.eval_expr(arr, env)?;
                let idx_v = self.eval_expr(idx, env)?;
                match (arr_v, idx_v) {
                    (Value::Array(a), Value::Int(i)) => {
                        let borrow = a.borrow();
                        let idx = if i < 0 { (borrow.len() as i64 + i) as usize } else { i as usize };
                        borrow.get(idx).cloned().ok_or_else(|| RuntimeError::at("index out of bounds", s.line))
                    }
                    (Value::Str(s2), Value::Int(i)) => {
                        let idx = if i < 0 { (s2.len() as i64 + i) as usize } else { i as usize };
                        s2.chars().nth(idx)
                            .map(|c| Value::Str(c.to_string()))
                            .ok_or_else(|| RuntimeError::at("string index out of bounds", s.line))
                    }
                    (Value::Map(m), Value::Str(key)) => {
                        m.borrow().get(&key).cloned().ok_or_else(|| RuntimeError::at(format!("key '{}' not found", key), s.line))
                    }
                    _ => Err(RuntimeError::at("invalid index operation", s.line)),
                }
            }

            Expr::StructInit(name, fields, _) => {
                let mut map = HashMap::new();
                for (fname, fexpr) in fields {
                    map.insert(fname.clone(), self.eval_expr(fexpr, env)?);
                }
                Ok(Value::Struct { name: name.clone(), fields: Rc::new(RefCell::new(map)) })
            }

            Expr::EnumVariant(enum_name, variant, args, _) => {
                let payload: Result<Vec<_>, _> = args.iter().map(|e| self.eval_expr(e, env)).collect();
                Ok(Value::EnumVariant { enum_name: enum_name.clone(), variant: variant.clone(), payload: payload? })
            }

            Expr::If(b) => {
                let cond = self.eval_expr(&b.cond, env)?;
                if self.is_truthy(&cond) {
                    self.eval_expr(&b.then_val, env)
                } else {
                    self.eval_expr(&b.else_val, env)
                }
            }

            Expr::Match(m) => self.eval_match(m, env),

            Expr::Block(stmts, tail, _) => {
                let child = Env::child(env);
                let _ = self.exec_block(stmts, &child)?;
                if let Some(tail_expr) = tail {
                    self.eval_expr(tail_expr, &child)
                } else {
                    Ok(Value::Unit)
                }
            }

            Expr::Range(start, end, _inclusive, _) => {
                let s = self.eval_expr(start, env)?;
                let e = self.eval_expr(end, env)?;
                match (s, e) {
                    (Value::Int(a), Value::Int(b)) => {
                        let items: Vec<Value> = (a..b).map(Value::Int).collect();
                        Ok(Value::Array(Rc::new(RefCell::new(items))))
                    }
                    _ => Err(RuntimeError::new("range requires integer bounds")),
                }
            }

            Expr::Closure(params, _, body, _) => {
                Ok(Value::Fn { params: params.clone(), body: vec![Stmt::Return(Some(*body.clone()), crate::tru_lang::lexer::Span { line: 0, col: 0 })], env: env.clone() })
            }

            Expr::Map(pairs, _) => {
                let mut map = HashMap::new();
                for (k, v) in pairs {
                    let key = match self.eval_expr(k, env)? {
                        Value::Str(s) => s,
                        Value::Int(n) => n.to_string(),
                        other => format!("{}", other),
                    };
                    map.insert(key, self.eval_expr(v, env)?);
                }
                Ok(Value::Map(Rc::new(RefCell::new(map))))
            }
        }
    }

    // ── binary ops ───────────────────────────────────────────────────────────

    fn eval_binary(&mut self, lhs: &Expr, op: &BinOp, rhs: &Expr, line: usize, env: &Env) -> IResult {
        let l = self.eval_expr(lhs, env)?;
        let r = self.eval_expr(rhs, env)?;

        match op {
            BinOp::Add => match (&l, &r) {
                (Value::Int(a),   Value::Int(b))   => Ok(Value::Int(a + b)),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
                (Value::Int(a),   Value::Float(b)) => Ok(Value::Float(*a as f64 + b)),
                (Value::Float(a), Value::Int(b))   => Ok(Value::Float(a + *b as f64)),
                (Value::Str(a),   Value::Str(b))   => Ok(Value::Str(format!("{}{}", a, b))),
                _ => Err(RuntimeError::at(format!("cannot add {:?} and {:?}", l, r), line)),
            },
            BinOp::Sub => self.arith(l, r, op, line),
            BinOp::Mul => self.arith(l, r, op, line),
            BinOp::Div => self.arith(l, r, op, line),
            BinOp::Rem => self.arith(l, r, op, line),
            BinOp::Eq  => Ok(Value::Bool(l == r)),
            BinOp::Ne  => Ok(Value::Bool(l != r)),
            BinOp::Lt  => self.cmp(l, r, op, line),
            BinOp::Le  => self.cmp(l, r, op, line),
            BinOp::Gt  => self.cmp(l, r, op, line),
            BinOp::Ge  => self.cmp(l, r, op, line),
            BinOp::And => Ok(Value::Bool(self.is_truthy(&l) && self.is_truthy(&r))),
            BinOp::Or  => Ok(Value::Bool(self.is_truthy(&l) || self.is_truthy(&r))),
            BinOp::BitAnd => match (&l, &r) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a & b)),
                _ => Err(RuntimeError::at("bitwise AND requires integers", line)),
            },
            BinOp::BitOr => match (&l, &r) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a | b)),
                _ => Err(RuntimeError::at("bitwise OR requires integers", line)),
            },
            BinOp::BitXor => match (&l, &r) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a ^ b)),
                _ => Err(RuntimeError::at("bitwise XOR requires integers", line)),
            },
            BinOp::Shl => match (&l, &r) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a << b)),
                _ => Err(RuntimeError::at("shift requires integers", line)),
            },
            BinOp::Shr => match (&l, &r) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a >> b)),
                _ => Err(RuntimeError::at("shift requires integers", line)),
            },
        }
    }

    fn arith(&self, l: Value, r: Value, op: &BinOp, line: usize) -> IResult {
        match (&l, &r) {
            (Value::Int(a), Value::Int(b)) => match op {
                BinOp::Sub => Ok(Value::Int(a - b)),
                BinOp::Mul => Ok(Value::Int(a * b)),
                BinOp::Div => if *b == 0 { Err(RuntimeError::at("division by zero", line)) }
                              else { Ok(Value::Int(a / b)) },
                BinOp::Rem => Ok(Value::Int(a % b)),
                _ => unreachable!(),
            },
            (Value::Float(a), Value::Float(b)) => match op {
                BinOp::Sub => Ok(Value::Float(a - b)),
                BinOp::Mul => Ok(Value::Float(a * b)),
                BinOp::Div => Ok(Value::Float(a / b)),
                BinOp::Rem => Ok(Value::Float(a % b)),
                _ => unreachable!(),
            },
            (Value::Int(a), Value::Float(b)) => match op {
                BinOp::Sub => Ok(Value::Float(*a as f64 - b)),
                BinOp::Mul => Ok(Value::Float(*a as f64 * b)),
                BinOp::Div => Ok(Value::Float(*a as f64 / b)),
                BinOp::Rem => Ok(Value::Float(*a as f64 % b)),
                _ => unreachable!(),
            },
            (Value::Float(a), Value::Int(b)) => match op {
                BinOp::Sub => Ok(Value::Float(a - *b as f64)),
                BinOp::Mul => Ok(Value::Float(a * *b as f64)),
                BinOp::Div => Ok(Value::Float(a / *b as f64)),
                BinOp::Rem => Ok(Value::Float(a % *b as f64)),
                _ => unreachable!(),
            },
            _ => Err(RuntimeError::at(format!("arithmetic type mismatch: {:?} {:?}", l, r), line)),
        }
    }

    fn cmp(&self, l: Value, r: Value, op: &BinOp, line: usize) -> IResult {
        let result = match (&l, &r) {
            (Value::Int(a),   Value::Int(b))   => match op { BinOp::Lt => a < b, BinOp::Le => a <= b, BinOp::Gt => a > b, BinOp::Ge => a >= b, _ => false },
            (Value::Float(a), Value::Float(b)) => match op { BinOp::Lt => a < b, BinOp::Le => a <= b, BinOp::Gt => a > b, BinOp::Ge => a >= b, _ => false },
            (Value::Str(a),   Value::Str(b))   => match op { BinOp::Lt => a < b, BinOp::Le => a <= b, BinOp::Gt => a > b, BinOp::Ge => a >= b, _ => false },
            _ => return Err(RuntimeError::at("comparison type mismatch", line)),
        };
        Ok(Value::Bool(result))
    }

    fn neg(&self, v: Value) -> IResult {
        match v {
            Value::Int(n)   => Ok(Value::Int(-n)),
            Value::Float(f) => Ok(Value::Float(-f)),
            _ => Err(RuntimeError::new("unary minus requires a number")),
        }
    }

    // ── assignment ───────────────────────────────────────────────────────────

    fn eval_assign(&mut self, lhs: &Expr, op: &AssignOp, rhs: &Expr, line: usize, env: &Env) -> IResult {
        let new_val = self.eval_expr(rhs, env)?;
        match lhs {
            Expr::Ident(name, _) => {
                let final_val = if *op == AssignOp::Assign {
                    new_val
                } else {
                    let old = env.get(name).ok_or_else(|| RuntimeError::at(format!("undefined '{}'", name), line))?;
                    self.apply_assign_op(old, op, new_val, line)?
                };
                if !env.assign(name, final_val.clone()) {
                    env.define(name, final_val); // auto-define if not found
                }
                Ok(Value::Unit)
            }
            Expr::Index(arr_expr, idx_expr, _) => {
                let idx_val = self.eval_expr(idx_expr, env)?;
                let arr_val = self.eval_expr(arr_expr, env)?;
                if let (Value::Array(arr), Value::Int(i)) = (arr_val, idx_val) {
                    let mut a = arr.borrow_mut();
                    let idx = if i < 0 { (a.len() as i64 + i) as usize } else { i as usize };
                    if idx < a.len() { a[idx] = new_val; }
                }
                Ok(Value::Unit)
            }
            Expr::Field(obj_expr, field, _) => {
                let obj = self.eval_expr(obj_expr, env)?;
                if let Value::Struct { fields, .. } = obj {
                    fields.borrow_mut().insert(field.clone(), new_val);
                }
                Ok(Value::Unit)
            }
            _ => Err(RuntimeError::at("invalid assignment target", line)),
        }
    }

    fn apply_assign_op(&self, old: Value, op: &AssignOp, new: Value, line: usize) -> IResult {
        match op {
            AssignOp::AddAssign => match (&old, &new) {
                (Value::Int(a),   Value::Int(b))   => Ok(Value::Int(a + b)),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
                (Value::Str(a),   Value::Str(b))   => Ok(Value::Str(format!("{}{}", a, b))),
                _ => Err(RuntimeError::at("type mismatch in +=", line)),
            },
            AssignOp::SubAssign => match (&old, &new) {
                (Value::Int(a),   Value::Int(b))   => Ok(Value::Int(a - b)),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a - b)),
                _ => Err(RuntimeError::at("type mismatch in -=", line)),
            },
            AssignOp::MulAssign => match (&old, &new) {
                (Value::Int(a),   Value::Int(b))   => Ok(Value::Int(a * b)),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a * b)),
                _ => Err(RuntimeError::at("type mismatch in *=", line)),
            },
            AssignOp::DivAssign => match (&old, &new) {
                (Value::Int(a),   Value::Int(b))   => if *b == 0 { Err(RuntimeError::at("division by zero", line)) } else { Ok(Value::Int(a / b)) },
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a / b)),
                _ => Err(RuntimeError::at("type mismatch in /=", line)),
            },
            AssignOp::Assign => Ok(new),
        }
    }

    // ── function calls ────────────────────────────────────────────────────────

    fn eval_call(&mut self, callee: &Expr, args: &[Expr], line: usize, env: &Env) -> IResult {
        let callee_val = self.eval_expr(callee, env)?;
        let arg_vals: Result<Vec<_>, _> = args.iter().map(|a| self.eval_expr(a, env)).collect();
        let arg_vals = arg_vals?;

        match callee_val {
            Value::NativeFn(name) => self.call_native(&name, arg_vals, line),
            Value::Fn { params, body, env: closure_env } => {
                let call_env = Env::child(&closure_env);
                for (param, val) in params.iter().zip(arg_vals.iter()) {
                    call_env.define(&param.name, val.clone());
                }
                match self.exec_block(&body, &call_env)? {
                    Value::Return(v) => Ok(*v),
                    other => Ok(other),
                }
            }
            _ => Err(RuntimeError::at(format!("not callable: {}", callee_val), line)),
        }
    }

    fn eval_method(&mut self, obj_expr: &Expr, method: &str, args: &[Expr], line: usize, env: &Env) -> IResult {
        let obj = self.eval_expr(obj_expr, env)?;
        let mut arg_vals: Vec<Value> = args.iter().map(|a| self.eval_expr(a, env)).collect::<Result<_, _>>()?;

        match (&obj, method) {
            // Array methods
            (Value::Array(a), "push") => {
                let val = arg_vals.into_iter().next().ok_or_else(|| RuntimeError::at("push needs 1 arg", line))?;
                a.borrow_mut().push(val);
                Ok(Value::Unit)
            }
            (Value::Array(a), "pop") => {
                Ok(a.borrow_mut().pop().unwrap_or(Value::Nil))
            }
            (Value::Array(a), "len") => Ok(Value::Int(a.borrow().len() as i64)),
            (Value::Array(a), "contains") => {
                let target = arg_vals.into_iter().next().unwrap_or(Value::Nil);
                let found = a.borrow().iter().any(|v| v == &target);
                Ok(Value::Bool(found))
            }
            (Value::Array(a), "join") => {
                let sep = match arg_vals.into_iter().next() { Some(Value::Str(s)) => s, _ => "".to_string() };
                let joined = a.borrow().iter().map(|v| format!("{}", v)).collect::<Vec<_>>().join(&sep);
                Ok(Value::Str(joined))
            }
            (Value::Array(a), "get") => {
                let i = match arg_vals.into_iter().next() { Some(Value::Int(n)) => n, _ => return Err(RuntimeError::at("get needs int", line)) };
                let borrow = a.borrow();
                let idx = if i < 0 { (borrow.len() as i64 + i) as usize } else { i as usize };
                Ok(borrow.get(idx).cloned().unwrap_or(Value::Nil))
            }
            // String methods
            (Value::Str(s), "len")       => Ok(Value::Int(s.len() as i64)),
            (Value::Str(s), "to_upper")  => Ok(Value::Str(s.to_uppercase())),
            (Value::Str(s), "to_lower")  => Ok(Value::Str(s.to_lowercase())),
            (Value::Str(s), "trim")      => Ok(Value::Str(s.trim().to_string())),
            (Value::Str(s), "contains")  => {
                let needle = match arg_vals.into_iter().next() { Some(Value::Str(n)) => n, _ => return Err(RuntimeError::at("contains needs string", line)) };
                Ok(Value::Bool(s.contains(&*needle)))
            }
            (Value::Str(s), "starts_with") => {
                let prefix = match arg_vals.into_iter().next() { Some(Value::Str(p)) => p, _ => return Err(RuntimeError::at("starts_with needs string", line)) };
                Ok(Value::Bool(s.starts_with(&*prefix)))
            }
            (Value::Str(s), "ends_with") => {
                let suffix = match arg_vals.into_iter().next() { Some(Value::Str(p)) => p, _ => return Err(RuntimeError::at("ends_with needs string", line)) };
                Ok(Value::Bool(s.ends_with(&*suffix)))
            }
            (Value::Str(s), "split") => {
                let sep = match arg_vals.into_iter().next() { Some(Value::Str(p)) => p, _ => " ".to_string() };
                let parts: Vec<Value> = s.split(&*sep).map(|p| Value::Str(p.to_string())).collect();
                Ok(Value::Array(Rc::new(RefCell::new(parts))))
            }
            (Value::Str(s), "replace") => {
                let mut iter = arg_vals.into_iter();
                let from = match iter.next() { Some(Value::Str(p)) => p, _ => return Err(RuntimeError::at("replace needs 2 strings", line)) };
                let to   = match iter.next() { Some(Value::Str(p)) => p, _ => return Err(RuntimeError::at("replace needs 2 strings", line)) };
                Ok(Value::Str(s.replace(&*from, &*to)))
            }
            (Value::Str(s), "chars") => {
                let chars: Vec<Value> = s.chars().map(|c| Value::Str(c.to_string())).collect();
                Ok(Value::Array(Rc::new(RefCell::new(chars))))
            }
            // Map methods
            (Value::Map(m), "insert") => {
                let mut iter = arg_vals.into_iter();
                let key = match iter.next() { Some(Value::Str(k)) => k, _ => return Err(RuntimeError::at("map.insert key must be string", line)) };
                let val = iter.next().unwrap_or(Value::Nil);
                m.borrow_mut().insert(key, val);
                Ok(Value::Unit)
            }
            (Value::Map(m), "get") => {
                let key = match arg_vals.into_iter().next() { Some(Value::Str(k)) => k, _ => return Err(RuntimeError::at("map.get key must be string", line)) };
                Ok(m.borrow().get(&key).cloned().unwrap_or(Value::Nil))
            }
            (Value::Map(m), "contains_key") => {
                let key = match arg_vals.into_iter().next() { Some(Value::Str(k)) => k, _ => return Err(RuntimeError::at("contains_key needs string", line)) };
                Ok(Value::Bool(m.borrow().contains_key(&key)))
            }
            (Value::Map(m), "len") => Ok(Value::Int(m.borrow().len() as i64)),
            (Value::Map(m), "keys") => {
                let keys: Vec<Value> = m.borrow().keys().map(|k| Value::Str(k.clone())).collect();
                Ok(Value::Array(Rc::new(RefCell::new(keys))))
            }
            // Int/Float methods
            (Value::Int(n), "to_float")  => Ok(Value::Float(*n as f64)),
            (Value::Float(f), "to_int")  => Ok(Value::Int(*f as i64)),
            (Value::Int(n), "abs")       => Ok(Value::Int(n.abs())),
            (Value::Float(f), "abs")     => Ok(Value::Float(f.abs())),
            (Value::Float(f), "sqrt")    => Ok(Value::Float(f.sqrt())),
            (Value::Float(f), "floor")   => Ok(Value::Float(f.floor())),
            (Value::Float(f), "ceil")    => Ok(Value::Float(f.ceil())),
            (Value::Float(f), "round")   => Ok(Value::Float(f.round())),
            _ => Err(RuntimeError::at(format!("no method '{}' on {:?}", method, obj), line)),
        }
    }

    // ── match ─────────────────────────────────────────────────────────────────

    fn eval_match(&mut self, m: &MatchExpr, env: &Env) -> IResult {
        let subject = self.eval_expr(&m.subject, env)?;
        for arm in &m.arms {
            let arm_env = Env::child(env);
            if self.match_pattern(&arm.pattern, &subject, &arm_env) {
                if let Some(guard) = &arm.guard {
                    let g = self.eval_expr(guard, &arm_env)?;
                    if !self.is_truthy(&g) { continue; }
                }
                return self.eval_expr(&arm.body, &arm_env);
            }
        }
        Err(RuntimeError::new("non-exhaustive match"))
    }

    fn match_pattern(&self, pattern: &Pattern, value: &Value, env: &Env) -> bool {
        match (pattern, value) {
            (Pattern::Wildcard, _) => true,
            (Pattern::Ident(name), v) => { env.define(name, v.clone()); true }
            (Pattern::Int(n),    Value::Int(v))    => n == v,
            (Pattern::Float(f),  Value::Float(v))  => f == v,
            (Pattern::Str(s),    Value::Str(v))    => s == v,
            (Pattern::Bool(b),   Value::Bool(v))   => b == v,
            (Pattern::Nil,       Value::Nil)        => true,
            (Pattern::Tuple(ps), Value::Tuple(vs))  => {
                ps.len() == vs.len() && ps.iter().zip(vs.iter()).all(|(p, v)| self.match_pattern(p, v, env))
            }
            (Pattern::EnumVariant(_, variant, ps), Value::EnumVariant { variant: vv, payload, .. }) => {
                variant == vv && ps.len() == payload.len() &&
                ps.iter().zip(payload.iter()).all(|(p, v)| self.match_pattern(p, v, env))
            }
            (Pattern::Or(patterns), v) => patterns.iter().any(|p| self.match_pattern(p, v, env)),
            _ => false,
        }
    }

    // ── stdlib native functions ───────────────────────────────────────────────

    fn call_native(&mut self, name: &str, args: Vec<Value>, line: usize) -> IResult {
        match name {
            "println" => {
                let s = args.iter().map(|v| format!("{}", v)).collect::<Vec<_>>().join(" ");
                self.output.push(s.clone());
                println!("{}", s);
                Ok(Value::Unit)
            }
            "print" => {
                let s = args.iter().map(|v| format!("{}", v)).collect::<Vec<_>>().join(" ");
                self.output.push(s.clone());
                print!("{}", s);
                Ok(Value::Unit)
            }
            "len" => match args.into_iter().next() {
                Some(Value::Array(a)) => Ok(Value::Int(a.borrow().len() as i64)),
                Some(Value::Str(s))   => Ok(Value::Int(s.len() as i64)),
                Some(Value::Map(m))   => Ok(Value::Int(m.borrow().len() as i64)),
                _ => Err(RuntimeError::at("len() requires array/string/map", line)),
            },
            "push" => {
                if let (Some(Value::Array(a)), Some(val)) = (args.get(0).cloned(), args.get(1).cloned()) {
                    a.borrow_mut().push(val);
                }
                Ok(Value::Unit)
            }
            "pop" => match args.into_iter().next() {
                Some(Value::Array(a)) => Ok(a.borrow_mut().pop().unwrap_or(Value::Nil)),
                _ => Err(RuntimeError::at("pop() requires array", line)),
            },
            "to_str" => match args.into_iter().next() {
                Some(v) => Ok(Value::Str(format!("{}", v))),
                None    => Ok(Value::Str(String::new())),
            },
            "to_int" => match args.into_iter().next() {
                Some(Value::Int(n))   => Ok(Value::Int(n)),
                Some(Value::Float(f)) => Ok(Value::Int(f as i64)),
                Some(Value::Str(s))   => s.parse::<i64>().map(Value::Int).map_err(|_| RuntimeError::at("cannot parse int", line)),
                Some(Value::Bool(b))  => Ok(Value::Int(if b { 1 } else { 0 })),
                _ => Err(RuntimeError::at("to_int() type error", line)),
            },
            "to_float" => match args.into_iter().next() {
                Some(Value::Float(f)) => Ok(Value::Float(f)),
                Some(Value::Int(n))   => Ok(Value::Float(n as f64)),
                Some(Value::Str(s))   => s.parse::<f64>().map(Value::Float).map_err(|_| RuntimeError::at("cannot parse float", line)),
                _ => Err(RuntimeError::at("to_float() type error", line)),
            },
            "assert" => {
                let ok = match args.first() { Some(Value::Bool(b)) => *b, _ => false };
                if !ok { Err(RuntimeError::at("assertion failed", line)) } else { Ok(Value::Unit) }
            }
            "assert_eq" => {
                let eq = args.get(0) == args.get(1);
                if !eq {
                    Err(RuntimeError::at(format!("assert_eq failed: {:?} != {:?}", args.get(0), args.get(1)), line))
                } else { Ok(Value::Unit) }
            }
            "range" => {
                let (start, end) = match (args.get(0), args.get(1)) {
                    (Some(Value::Int(s)), Some(Value::Int(e))) => (*s, *e),
                    (Some(Value::Int(e)), None) => (0, *e),
                    _ => return Err(RuntimeError::at("range() requires int args", line)),
                };
                let items: Vec<Value> = (start..end).map(Value::Int).collect();
                Ok(Value::Array(Rc::new(RefCell::new(items))))
            }
            "type_of" => match args.first() {
                Some(Value::Int(_))   => Ok(Value::Str("int".to_string())),
                Some(Value::Float(_)) => Ok(Value::Str("float".to_string())),
                Some(Value::Bool(_))  => Ok(Value::Str("bool".to_string())),
                Some(Value::Str(_))   => Ok(Value::Str("str".to_string())),
                Some(Value::Nil)      => Ok(Value::Str("nil".to_string())),
                Some(Value::Array(_)) => Ok(Value::Str("array".to_string())),
                Some(Value::Map(_))   => Ok(Value::Str("map".to_string())),
                Some(Value::Fn { .. }) | Some(Value::NativeFn(_)) => Ok(Value::Str("fn".to_string())),
                _ => Ok(Value::Str("unknown".to_string())),
            },
            "exit" => std::process::exit(0),
            other => {
                crate::tru_lang::stdlib::call_stdlib(other, args)
                    .map_err(|e| RuntimeError::at(e, line))
            }
        }
    }

    // ── utilities ─────────────────────────────────────────────────────────────

    fn is_truthy(&self, v: &Value) -> bool {
        match v {
            Value::Bool(false) | Value::Nil | Value::Unit => false,
            Value::Int(0)  => false,
            Value::Str(s) if s.is_empty() => false,
            _ => true,
        }
    }

    fn collect_iterable(&self, v: Value) -> Result<Vec<Value>, RuntimeError> {
        match v {
            Value::Array(a) => Ok(a.borrow().clone()),
            Value::Str(s) => Ok(s.chars().map(|c| Value::Str(c.to_string())).collect()),
            _ => Err(RuntimeError::new("value is not iterable")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tru_lang::parser::Parser;

    fn run(src: &str) -> Value {
        let prog = Parser::parse(src).expect("parse failed");
        let mut interp = Interpreter::new();
        interp.run(&prog).expect("runtime error")
    }

    #[test]
    fn test_arithmetic() {
        assert_eq!(run("2 + 3 * 4"), Value::Int(14));
    }

    #[test]
    fn test_variables() {
        assert_eq!(run("let x = 10; let y = 20; x + y"), Value::Int(30));
    }

    #[test]
    fn test_function() {
        let src = r#"
fn double(n: i64) -> i64 { return n * 2; }
double(21)
"#;
        assert_eq!(run(src), Value::Int(42));
    }

    #[test]
    fn test_if_else() {
        assert_eq!(run("if true { 1 } else { 2 }"), Value::Int(1));
        assert_eq!(run("if false { 1 } else { 2 }"), Value::Int(2));
    }

    #[test]
    fn test_while_loop() {
        let src = r#"
let mut i = 0;
let mut sum = 0;
while i < 10 { sum += i; i += 1; }
sum
"#;
        assert_eq!(run(src), Value::Int(45));
    }

    #[test]
    fn test_array() {
        let src = r#"
let arr = [1, 2, 3];
arr[0] + arr[1] + arr[2]
"#;
        assert_eq!(run(src), Value::Int(6));
    }

    #[test]
    fn test_string_concat() {
        assert_eq!(run(r#""hello" + " " + "world""#), Value::Str("hello world".to_string()));
    }
}
