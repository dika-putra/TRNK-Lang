//! tru_id — TRU Language core module
//!
//! Language pipeline:
//!   source (.tru) → Lexer → Parser → AST → Interpreter  (direct execution)
//!                                         → Compiler     (Rust transpilation)

pub mod lexer;
pub mod ast;
pub mod parser;
pub mod interpreter;
pub mod compiler;
pub mod stdlib;
pub mod scaffold;

// ─── Re-exports ───────────────────────────────────────────────────────────────

pub use lexer::{Lexer, Token, TokenKind};
pub use ast::{Program, Stmt, Expr, BinOp, UnOp};
pub use parser::Parser;
pub use interpreter::{Interpreter, Value, Env};
pub use compiler::{Compiler, compile_tru};
pub use scaffold::new_project;

// ─── High-level API ───────────────────────────────────────────────────────────

use std::path::Path;

/// Result type used throughout the language crate.
pub type TruResult<T> = Result<T, TruError>;

/// Unified error type covering all language phases.
#[derive(Debug)]
pub enum TruError {
    Lex(lexer::LexError),
    Parse(parser::ParseError),
    Runtime(interpreter::RuntimeError),
    Compile(compiler::CompileError),
    Io(std::io::Error),
}

impl std::fmt::Display for TruError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TruError::Lex(e)     => write!(f, "{}", e),
            TruError::Parse(e)   => write!(f, "{}", e),
            TruError::Runtime(e) => write!(f, "{}", e),
            TruError::Compile(e) => write!(f, "{}", e),
            TruError::Io(e)      => write!(f, "[IoError] {}", e),
        }
    }
}

impl From<parser::ParseError>       for TruError { fn from(e: parser::ParseError)       -> Self { TruError::Parse(e) } }
impl From<interpreter::RuntimeError> for TruError { fn from(e: interpreter::RuntimeError) -> Self { TruError::Runtime(e) } }
impl From<compiler::CompileError>   for TruError { fn from(e: compiler::CompileError)   -> Self { TruError::Compile(e) } }
impl From<std::io::Error>           for TruError { fn from(e: std::io::Error)           -> Self { TruError::Io(e) } }

/// Run TRU source code directly (interpreter mode).
pub fn run_source(source: &str) -> TruResult<Value> {
    let program = Parser::parse(source)?;
    let mut interp = Interpreter::new();
    let result = interp.run(&program)?;
    Ok(result)
}

/// Run a .tru file directly.
pub fn run_file<P: AsRef<Path>>(path: P) -> TruResult<Value> {
    let path = path.as_ref();
    let source = std::fs::read_to_string(path)?;
    let source = expand_local_uses(&source, path.parent().unwrap_or(Path::new(".")))?;
    run_source(&source)
}

/// Replaces each `use modules::<name>;` line with the contents of `<dir>/modules/<name>.tru`.
// ponytail: textual include, one level, shared global scope, and error line numbers shift
// after an include. Replace with a real module system (scoped `mod`, `a::b()` calls) when needed.
fn expand_local_uses(source: &str, dir: &Path) -> std::io::Result<String> {
    use std::io::{Error, ErrorKind};
    let mut out = String::with_capacity(source.len());
    for line in source.lines() {
        let Some(rest) = line.trim().strip_prefix("use modules::") else {
            out.push_str(line);
            out.push('\n');
            continue;
        };
        let name = rest
            .strip_suffix(';')
            .map(str::trim)
            .filter(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'))
            .ok_or_else(|| Error::new(ErrorKind::InvalidInput, format!("invalid module import `{}`, expected `use modules::<name>;`", line.trim())))?;
        let file = dir.join("modules").join(format!("{name}.tru"));
        let text = std::fs::read_to_string(&file)
            .map_err(|e| Error::new(e.kind(), format!("cannot load module {}: {e}", file.display())))?;
        out.push_str(&text);
        out.push('\n');
    }
    Ok(out)
}

/// Transpile TRU source to Rust source.
pub fn transpile_source(source: &str) -> TruResult<String> {
    Ok(compile_tru(source)?)
}

/// Transpile a .tru file to a Rust source string.
pub fn transpile_file<P: AsRef<Path>>(path: P) -> TruResult<String> {
    let source = std::fs::read_to_string(&path)?;
    transpile_source(&source)
}

/// Transpile a .tru file and write the output as a .rs file alongside it.
pub fn transpile_to_rs<P: AsRef<Path>>(tru_path: P) -> TruResult<std::path::PathBuf> {
    let tru_path = tru_path.as_ref();
    let rust_src = transpile_file(tru_path)?;
    let out_path = tru_path.with_extension("rs");
    std::fs::write(&out_path, &rust_src)?;
    Ok(out_path)
}

// ─── REPL ─────────────────────────────────────────────────────────────────────

/// Interactive REPL for the TRU language.
pub struct TruRepl {
    interp: Interpreter,
    env: Env,
    history: Vec<String>,
}

impl TruRepl {
    pub fn new() -> Self {
        let env = Env::new();
        stdlib::load(&env);
        let mut interp = Interpreter::new();
        interp.load_stdlib(&env);
        Self {
            interp,
            env,
            history: Vec::new(),
        }
    }

    /// Evaluate a single line / snippet in the REPL session.
    pub fn eval(&mut self, line: &str) -> TruResult<Value> {
        self.history.push(line.to_string());
        let program = Parser::parse(line)?;
        // Register fn declarations in persistent env
        for stmt in &program.stmts {
            if let Stmt::Fn(f) = stmt {
                self.env.define(&f.name, Value::Fn {
                    params: f.params.clone(),
                    body: f.body.clone(),
                    env: self.env.clone(),
                });
            }
        }
        let result = self.interp.exec_block_in_env(&program.stmts, &self.env)?;
        Ok(result)
    }

    pub fn history(&self) -> &[String] {
        &self.history
    }

    pub fn output(&self) -> &[String] {
        &self.interp.output
    }
}

// ─── Interpreter helper (exec in existing env) ────────────────────────────────

impl Interpreter {
    pub fn exec_block_in_env(&mut self, stmts: &[ast::Stmt], env: &Env) -> Result<Value, interpreter::RuntimeError> {
        self.exec_block(stmts, env)
    }
}

// ─── Language info ────────────────────────────────────────────────────────────

pub const TRU_VERSION:    &str = "0.1.0";
pub const TRU_EXTENSION:  &str = "tru";
pub const TRU_CRATE_NAME: &str = "tru_id_core";

/// Print a brief language description.
pub fn describe() -> String {
    format!(
        "tru_id v{} — TRU Language\n\
         Extension : .{}\n\
         Crate     : {}\n\
         Paradigms : imperative, functional, struct-based\n\
         Backend   : tree-walk interpreter + Rust transpiler\n\
         Stdlib    : math, io, string, array, map, sys",
        TRU_VERSION, TRU_EXTENSION, TRU_CRATE_NAME
    )
}
