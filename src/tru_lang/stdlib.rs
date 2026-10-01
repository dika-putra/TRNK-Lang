//! TRU Language Standard Library
//! Registers built-in modules available in every .tru program.

use std::collections::HashMap;
use crate::tru_lang::interpreter::{Env, Value};

/// Load all standard library modules into an environment.
pub fn load(env: &Env) {
    load_math(env);
    load_io(env);
    load_string(env);
    load_array(env);
    load_map(env);
    load_sys(env);
}

// ─── math ─────────────────────────────────────────────────────────────────────

fn load_math(env: &Env) {
    // Constants
    env.define("PI",  Value::Float(std::f64::consts::PI));
    env.define("E",   Value::Float(std::f64::consts::E));
    env.define("TAU", Value::Float(std::f64::consts::TAU));
    env.define("INF", Value::Float(f64::INFINITY));
    env.define("NAN", Value::Float(f64::NAN));
    env.define("I64_MAX", Value::Int(i64::MAX));
    env.define("I64_MIN", Value::Int(i64::MIN));

    // Native functions (resolved by interpreter)
    for name in &[
        "sqrt", "abs", "floor", "ceil", "round",
        "sin", "cos", "tan", "ln", "log2", "log10",
        "pow", "min", "max", "clamp",
    ] {
        env.define(name, Value::NativeFn(format!("math::{}", name)));
    }
}

// ─── io ───────────────────────────────────────────────────────────────────────

fn load_io(env: &Env) {
    for name in &["read_line", "read_file", "write_file", "file_exists"] {
        env.define(name, Value::NativeFn(format!("io::{}", name)));
    }
}

// ─── string helpers ───────────────────────────────────────────────────────────

fn load_string(env: &Env) {
    for name in &["format", "parse_int", "parse_float", "char_at", "repeat"] {
        env.define(name, Value::NativeFn(format!("str::{}", name)));
    }
}

// ─── array helpers ────────────────────────────────────────────────────────────

fn load_array(env: &Env) {
    for name in &["sort", "reverse", "map", "filter", "reduce", "zip", "enumerate", "flatten"] {
        env.define(name, Value::NativeFn(format!("arr::{}", name)));
    }
}

// ─── map helpers ──────────────────────────────────────────────────────────────

fn load_map(env: &Env) {
    for name in &["new_map", "map_keys", "map_values", "map_entries"] {
        env.define(name, Value::NativeFn(format!("map::{}", name)));
    }
}

// ─── sys ──────────────────────────────────────────────────────────────────────

fn load_sys(env: &Env) {
    for name in &["env_var", "args", "cwd", "sleep_ms"] {
        env.define(name, Value::NativeFn(format!("sys::{}", name)));
    }
}

/// Dispatch extended standard library native calls from the interpreter.
pub fn call_stdlib(name: &str, args: Vec<Value>) -> Result<Value, String> {
    match name {
        // math
        "math::sqrt"  => num1(&args, |f| f.sqrt()),
        "math::abs"   => num1_either(&args, |i| i.abs(), |f| f.abs()),
        "math::floor" => num1(&args, |f| f.floor()),
        "math::ceil"  => num1(&args, |f| f.ceil()),
        "math::round" => num1(&args, |f| f.round()),
        "math::sin"   => num1(&args, |f| f.sin()),
        "math::cos"   => num1(&args, |f| f.cos()),
        "math::tan"   => num1(&args, |f| f.tan()),
        "math::ln"    => num1(&args, |f| f.ln()),
        "math::log2"  => num1(&args, |f| f.log2()),
        "math::log10" => num1(&args, |f| f.log10()),
        "math::pow"   => {
            match (args.get(0), args.get(1)) {
                (Some(Value::Float(a)), Some(Value::Float(b))) => Ok(Value::Float(a.powf(*b))),
                (Some(Value::Int(a)),   Some(Value::Int(b)))   => Ok(Value::Int(a.pow(*b as u32))),
                (Some(Value::Float(a)), Some(Value::Int(b)))   => Ok(Value::Float(a.powi(*b as i32))),
                _ => Err("pow requires numeric args".to_string()),
            }
        }
        "math::min"  => {
            match (args.get(0), args.get(1)) {
                (Some(Value::Int(a)),   Some(Value::Int(b)))   => Ok(Value::Int(*a.min(b))),
                (Some(Value::Float(a)), Some(Value::Float(b))) => Ok(Value::Float(a.min(*b))),
                _ => Err("min requires matching numeric types".to_string()),
            }
        }
        "math::max"  => {
            match (args.get(0), args.get(1)) {
                (Some(Value::Int(a)),   Some(Value::Int(b)))   => Ok(Value::Int(*a.max(b))),
                (Some(Value::Float(a)), Some(Value::Float(b))) => Ok(Value::Float(a.max(*b))),
                _ => Err("max requires matching numeric types".to_string()),
            }
        }
        "math::clamp" => {
            match (args.get(0), args.get(1), args.get(2)) {
                (Some(Value::Int(v)), Some(Value::Int(lo)), Some(Value::Int(hi))) => Ok(Value::Int((*v).clamp(*lo, *hi))),
                (Some(Value::Float(v)), Some(Value::Float(lo)), Some(Value::Float(hi))) => Ok(Value::Float(v.clamp(*lo, *hi))),
                _ => Err("clamp requires 3 numeric args of same type".to_string()),
            }
        }

        // string
        "str::format" => {
            let parts: Vec<String> = args.iter().map(|v| format!("{}", v)).collect();
            Ok(Value::Str(parts.join("")))
        }
        "str::parse_int" => match args.first() {
            Some(Value::Str(s)) => s.trim().parse::<i64>().map(Value::Int).map_err(|e| e.to_string()),
            _ => Err("parse_int requires a string".to_string()),
        },
        "str::parse_float" => match args.first() {
            Some(Value::Str(s)) => s.trim().parse::<f64>().map(Value::Float).map_err(|e| e.to_string()),
            _ => Err("parse_float requires a string".to_string()),
        },
        "str::char_at" => match (args.get(0), args.get(1)) {
            (Some(Value::Str(s)), Some(Value::Int(i))) => {
                let idx = *i as usize;
                s.chars().nth(idx).map(|c| Value::Str(c.to_string())).ok_or_else(|| "index out of bounds".to_string())
            }
            _ => Err("char_at(str, int)".to_string()),
        },
        "str::repeat" => match (args.get(0), args.get(1)) {
            (Some(Value::Str(s)), Some(Value::Int(n))) => Ok(Value::Str(s.repeat(*n as usize))),
            _ => Err("repeat(str, int)".to_string()),
        },

        // array
        "arr::sort" => match args.first() {
            Some(Value::Array(a)) => {
                let mut v = a.borrow().clone();
                v.sort_by(|x, y| match (x, y) {
                    (Value::Int(a), Value::Int(b))     => a.cmp(b),
                    (Value::Float(a), Value::Float(b)) => a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal),
                    (Value::Str(a), Value::Str(b))     => a.cmp(b),
                    _ => std::cmp::Ordering::Equal,
                });
                *a.borrow_mut() = v;
                Ok(Value::Unit)
            }
            _ => Err("sort requires an array".to_string()),
        },
        "arr::reverse" => match args.first() {
            Some(Value::Array(a)) => { a.borrow_mut().reverse(); Ok(Value::Unit) }
            _ => Err("reverse requires an array".to_string()),
        },
        "arr::flatten" => match args.first() {
            Some(Value::Array(outer)) => {
                let mut flat = Vec::new();
                for item in outer.borrow().iter() {
                    match item {
                        Value::Array(inner) => flat.extend(inner.borrow().clone()),
                        other => flat.push(other.clone()),
                    }
                }
                Ok(Value::Array(std::rc::Rc::new(std::cell::RefCell::new(flat))))
            }
            _ => Err("flatten requires an array".to_string()),
        },
        "arr::enumerate" => match args.first() {
            Some(Value::Array(a)) => {
                let pairs: Vec<Value> = a.borrow().iter().enumerate().map(|(i, v)| {
                    Value::Tuple(vec![Value::Int(i as i64), v.clone()])
                }).collect();
                Ok(Value::Array(std::rc::Rc::new(std::cell::RefCell::new(pairs))))
            }
            _ => Err("enumerate requires an array".to_string()),
        },

        // map
        "map::new_map" => Ok(Value::Map(std::rc::Rc::new(std::cell::RefCell::new(HashMap::new())))),
        "map::map_keys" => match args.first() {
            Some(Value::Map(m)) => {
                let keys: Vec<Value> = m.borrow().keys().cloned().map(Value::Str).collect();
                Ok(Value::Array(std::rc::Rc::new(std::cell::RefCell::new(keys))))
            }
            _ => Err("map_keys requires a map".to_string()),
        },
        "map::map_values" => match args.first() {
            Some(Value::Map(m)) => {
                let vals: Vec<Value> = m.borrow().values().cloned().collect();
                Ok(Value::Array(std::rc::Rc::new(std::cell::RefCell::new(vals))))
            }
            _ => Err("map_values requires a map".to_string()),
        },
        "map::map_entries" => match args.first() {
            Some(Value::Map(m)) => {
                let entries: Vec<Value> = m.borrow().iter().map(|(k, v)| {
                    Value::Tuple(vec![Value::Str(k.clone()), v.clone()])
                }).collect();
                Ok(Value::Array(std::rc::Rc::new(std::cell::RefCell::new(entries))))
            }
            _ => Err("map_entries requires a map".to_string()),
        },

        // sys
        "sys::env_var" => match args.first() {
            Some(Value::Str(name)) => Ok(std::env::var(name).map(Value::Str).unwrap_or(Value::Nil)),
            _ => Err("env_var requires a string key".to_string()),
        },
        "sys::args" => {
            let args_vals: Vec<Value> = std::env::args().map(Value::Str).collect();
            Ok(Value::Array(std::rc::Rc::new(std::cell::RefCell::new(args_vals))))
        },
        "sys::cwd" => {
            let path = std::env::current_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
            Ok(Value::Str(path))
        },
        "sys::sleep_ms" => {
            if let Some(Value::Int(ms)) = args.first() {
                std::thread::sleep(std::time::Duration::from_millis(*ms as u64));
            }
            Ok(Value::Unit)
        },

        // io
        "io::read_line" => {
            let mut buf = String::new();
            std::io::stdin().read_line(&mut buf).ok();
            Ok(Value::Str(buf.trim_end_matches('\n').to_string()))
        },
        "io::read_file" => match args.first() {
            Some(Value::Str(path)) => std::fs::read_to_string(path).map(Value::Str).map_err(|e| e.to_string()),
            _ => Err("read_file requires a string path".to_string()),
        },
        "io::write_file" => match (args.get(0), args.get(1)) {
            (Some(Value::Str(path)), Some(Value::Str(content))) => {
                std::fs::write(path, content).map(|_| Value::Unit).map_err(|e| e.to_string())
            }
            _ => Err("write_file(path: str, content: str)".to_string()),
        },
        "io::file_exists" => match args.first() {
            Some(Value::Str(path)) => Ok(Value::Bool(std::path::Path::new(path).exists())),
            _ => Err("file_exists requires a string path".to_string()),
        },

        other => Err(format!("unknown stdlib function '{}'", other)),
    }
}

// ─── helpers ──────────────────────────────────────────────────────────────────

fn num1(args: &[Value], f: impl Fn(f64) -> f64) -> Result<Value, String> {
    match args.first() {
        Some(Value::Float(n)) => Ok(Value::Float(f(*n))),
        Some(Value::Int(n))   => Ok(Value::Float(f(*n as f64))),
        _ => Err("expected a number".to_string()),
    }
}

fn num1_either(args: &[Value], fi: impl Fn(i64) -> i64, ff: impl Fn(f64) -> f64) -> Result<Value, String> {
    match args.first() {
        Some(Value::Int(n))   => Ok(Value::Int(fi(*n))),
        Some(Value::Float(n)) => Ok(Value::Float(ff(*n))),
        _ => Err("expected a number".to_string()),
    }
}
