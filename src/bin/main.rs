//! tru_id — TRU Language command-line runner
//!
//! Usage:
//!   tru_id run   <file.tru>          # interpret a .tru file
//!   tru_id build <file.tru>          # transpile .tru → .rs
//!   tru_id repl                      # interactive REPL
//!   tru_id check <file.tru>          # parse + type-check only
//!   tru_id new   <name>              # create a project boilerplate
//!   tru_id version                   # show language version

use std::io::{self, BufRead, Write};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    match args.get(1).map(|s| s.as_str()) {
        Some("run")   => cmd_run(&args),
        Some("build") => cmd_build(&args),
        Some("repl")  => cmd_repl(),
        Some("check") => cmd_check(&args),
        Some("new")   => cmd_new(&args),
        Some("version") | Some("--version") | Some("-v") => cmd_version(),
        Some("help")  | Some("--help")      | Some("-h") => cmd_help(),
        _ => { cmd_help(); std::process::exit(1); }
    }
}

fn cmd_version() {
    println!("tru_id v{}", tru_id_core::TRU_VERSION);
    println!("crate   : {}", tru_id_core::TRU_CRATE_NAME);
    println!("ext     : .{}", tru_id_core::TRU_EXTENSION);
}

fn cmd_help() {
    println!("tru_id — TRU Language runner\n");
    println!("USAGE:");
    println!("  tru_id run   <file.tru>    Run a TRU program");
    println!("  tru_id build <file.tru>    Transpile TRU → Rust (.rs)");
    println!("  tru_id repl                Interactive REPL");
    println!("  tru_id check <file.tru>    Parse and check without running");
    println!("  tru_id new   <name>        Create a project with fetch/train/inference/pipe modules");
    println!("  tru_id version             Show version info");
}

fn cmd_run(args: &[String]) {
    let path = match args.get(2) {
        Some(p) => p,
        None => { eprintln!("error: expected a .tru file path"); std::process::exit(1); }
    };
    match tru_id_core::run_file(path) {
        Ok(val) => {
            if !matches!(val, tru_id_core::Value::Unit) {
                println!("{}", val);
            }
        }
        Err(e) => { eprintln!("{}", e); std::process::exit(1); }
    }
}

fn cmd_build(args: &[String]) {
    let path = match args.get(2) {
        Some(p) => p,
        None => { eprintln!("error: expected a .tru file path"); std::process::exit(1); }
    };
    match tru_id_core::transpile_to_rs(path) {
        Ok(out) => println!("compiled → {}", out.display()),
        Err(e)  => { eprintln!("{}", e); std::process::exit(1); }
    }
}

fn cmd_new(args: &[String]) {
    let name = match args.get(2) {
        Some(n) => n,
        None => { eprintln!("error: expected a project name"); std::process::exit(1); }
    };
    match tru_id_core::new_project(name, std::path::Path::new(".")) {
        Ok(root) => {
            println!("created {}", root.display());
            println!("next:\n  cd {}\n  tru_id run src/main.tru", name);
        }
        Err(e) => { eprintln!("error: {}", e); std::process::exit(1); }
    }
}

fn cmd_check(args: &[String]) {
    let path = match args.get(2) {
        Some(p) => p,
        None => { eprintln!("error: expected a .tru file path"); std::process::exit(1); }
    };
    let source = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => { eprintln!("error reading file: {}", e); std::process::exit(1); }
    };
    match tru_id_core::tru_lang::Parser::parse(&source) {
        Ok(prog) => println!("ok — {} top-level statements", prog.stmts.len()),
        Err(e)   => { eprintln!("{}", e); std::process::exit(1); }
    }
}

fn cmd_repl() {
    println!("tru_id v{} REPL  (Ctrl-D or 'exit' to quit)", tru_id_core::TRU_VERSION);
    let mut repl = tru_id_core::TruRepl::new();
    let stdin = io::stdin();
    loop {
        print!("tru> ");
        io::stdout().flush().ok();

        let mut line = String::new();
        match stdin.lock().read_line(&mut line) {
            Ok(0) | Err(_) => { println!(); break; } // EOF
            Ok(_) => {}
        }
        let trimmed = line.trim();
        if trimmed == "exit" || trimmed == "quit" { break; }
        if trimmed.is_empty() { continue; }

        match repl.eval(trimmed) {
            Ok(tru_id_core::Value::Unit) => {}
            Ok(val) => println!("= {}", val),
            Err(e)  => eprintln!("{}", e),
        }
    }
    println!("bye.");
}
