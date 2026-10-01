//! TRUNg Build System
//! 
//! Build system that uses trunk.toml instead of Cargo.toml
//! Provides enhanced build options for TRU language and performance optimizations

use std::process::Command;
use std::path::{Path, PathBuf};
use std::fs;
use anyhow::{Result, Context};
use crate::trung_config::*;

/// TRUNg Build System
pub struct TrungBuilder {
    config: TrungConfig,
    workspace_dir: PathBuf,
}

impl TrungBuilder {
    /// Create new TRUNg builder
    pub fn new<P: AsRef<Path>>(workspace_dir: P) -> Result<Self> {
        let workspace_dir = workspace_dir.as_ref().to_path_buf();
        let config_path = workspace_dir.join("trunk.toml");
        let config = TrungConfig::load_from_file(&config_path)?;
        
        Ok(Self {
            config,
            workspace_dir,
        })
    }
    
    /// Build the project
    pub fn build(&self, target: Option<&str>, profile: &str) -> Result<()> {
        println!("🚀 Building TRUNg project: {}", self.config.project.name);
        
        // Generate Cargo.toml for compatibility
        self.generate_cargo_toml()?;
        
        // Set optimization flags
        self.set_build_environment(profile)?;
        
        // Build with cargo
        let mut cmd = Command::new("cargo");
        cmd.arg("build");
        
        if profile == "release" {
            cmd.arg("--release");
        }
        
        if let Some(target) = target {
            cmd.arg("--target").arg(target);
        }
        
        cmd.current_dir(&self.workspace_dir);
        
        println!("📦 Running: {:?}", cmd);
        let output = cmd.output()
            .with_context(|| "Failed to execute cargo build")?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Build failed: {}", stderr);
        }
        
        println!("✅ Build completed successfully!");
        Ok(())
    }
    
    /// Run tests
    pub fn test(&self) -> Result<()> {
        println!("🧪 Running tests for TRUNg project: {}", self.config.project.name);
        
        self.generate_cargo_toml()?;
        
        let mut cmd = Command::new("cargo");
        cmd.arg("test");
        cmd.current_dir(&self.workspace_dir);
        
        println!("📦 Running: {:?}", cmd);
        let output = cmd.output()
            .with_context(|| "Failed to execute cargo test")?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Tests failed: {}", stderr);
        }
        
        println!("✅ All tests passed!");
        Ok(())
    }
    
    /// Build WASM target
    pub fn build_wasm(&self) -> Result<()> {
        println!("🌐 Building WASM for TRUNg project: {}", self.config.project.name);
        
        // Check if wasm target is installed
        let wasm_target_check = Command::new("rustup")
            .args(["target", "list", "--installed"])
            .output()?;
        
        let wasm_installed = String::from_utf8_lossy(&wasm_target_check.stdout)
            .contains("wasm32-unknown-unknown");
        
        if !wasm_installed {
            println!("📦 Installing WASM target...");
            let install_cmd = Command::new("rustup")
                .args(["target", "add", "wasm32-unknown-unknown"])
                .output()?;
            
            if !install_cmd.status.success() {
                anyhow::bail!("Failed to install WASM target");
            }
        }
        
        self.build(Some("wasm32-unknown-unknown"), "release")?;
        
        // Create pkg directory for wasm-pack
        let pkg_dir = self.workspace_dir.join("pkg");
        fs::create_dir_all(&pkg_dir)?;
        
        println!("✅ WASM build completed!");
        Ok(())
    }
    
    /// Run TRU compiler
    pub fn compile_tru<P: AsRef<Path>>(&self, input_file: P) -> Result<()> {
        println!("🔨 Compiling TRU file: {:?}", input_file.as_ref());
        
        let input_file = input_file.as_ref();
        let output_file = input_file.with_extension("rs");
        
        // Use TRU compiler
        let mut cmd = Command::new("cargo");
        cmd.args(["run", "--bin", "tru-compiler", "--"])
            .arg(input_file)
            .arg("-o")
            .arg(&output_file);
        
        cmd.current_dir(&self.workspace_dir);
        
        println!("📦 Running: {:?}", cmd);
        let output = cmd.output()
            .with_context(|| "Failed to execute TRU compiler")?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("TRU compilation failed: {}", stderr);
        }
        
        println!("✅ TRU compilation completed: {:?}", output_file);
        Ok(())
    }
    
    /// Run TRU program
    pub fn run_tru<P: AsRef<Path>>(&self, tru_file: P) -> Result<()> {
        println!("🏃 Running TRU program: {:?}", tru_file.as_ref());
        
        // Compile first
        self.compile_tru(&tru_file)?;
        
        // Run compiled program
        let mut cmd = Command::new("cargo");
        cmd.args(["run", "--bin", "trunk", "--"])
            .arg(tru_file.as_ref());
        
        cmd.current_dir(&self.workspace_dir);
        
        println!("📦 Running: {:?}", cmd);
        let output = cmd.output()
            .with_context(|| "Failed to run TRU program")?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("TRU program failed: {}", stderr);
        }
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        println!("📤 Output:\n{}", stdout);
        
        Ok(())
    }
    
    /// Install project
    pub fn install(&self) -> Result<()> {
        println!("📦 Installing TRUNg project: {}", self.config.project.name);
        
        // Build first
        self.build(None, "release")?;
        
        // Install with cargo
        let mut cmd = Command::new("cargo");
        cmd.args(["install", "--path", "."]);
        cmd.current_dir(&self.workspace_dir);
        
        println!("📦 Running: {:?}", cmd);
        let output = cmd.output()
            .with_context(|| "Failed to install project")?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Installation failed: {}", stderr);
        }
        
        println!("✅ Installation completed!");
        Ok(())
    }
    
    /// Clean build artifacts
    pub fn clean(&self) -> Result<()> {
        println!("🧹 Cleaning TRUNg project: {}", self.config.project.name);
        
        let mut cmd = Command::new("cargo");
        cmd.arg("clean");
        cmd.current_dir(&self.workspace_dir);
        
        println!("📦 Running: {:?}", cmd);
        let output = cmd.output()
            .with_context(|| "Failed to clean project")?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Clean failed: {}", stderr);
        }
        
        // Remove generated Cargo.toml
        let cargo_toml_path = self.workspace_dir.join("Cargo.toml");
        if cargo_toml_path.exists() {
            fs::remove_file(&cargo_toml_path)?;
        }
        
        // Remove pkg directory
        let pkg_dir = self.workspace_dir.join("pkg");
        if pkg_dir.exists() {
            fs::remove_dir_all(&pkg_dir)?;
        }
        
        println!("✅ Clean completed!");
        Ok(())
    }
    
    /// Generate Cargo.toml from trunk.toml for compatibility
    fn generate_cargo_toml(&self) -> Result<()> {
        let cargo_toml_path = self.workspace_dir.join("Cargo.toml");
        let cargo_toml_content = self.config.generate_cargo_toml()?;
        
        fs::write(&cargo_toml_path, cargo_toml_content)
            .with_context(|| "Failed to write Cargo.toml")?;
        
        Ok(())
    }
    
    /// Set build environment variables
    fn set_build_environment(&self, profile: &str) -> Result<()> {
        let flags = self.config.get_optimization_flags();
        let rustflags = flags.join(" ");
        
        std::env::set_var("RUSTFLAGS", rustflags);
        
        if profile == "release" {
            std::env::set_var("CARGO_PROFILE_RELEASE_LTO", "true");
            std::env::set_var("CARGO_PROFILE_RELEASE_CODEGEN_UNITS", "1");
            std::env::set_var("CARGO_PROFILE_RELEASE_OPT_LEVEL", "3");
            std::env::set_var("CARGO_PROFILE_RELEASE_PANIC", "abort");
            std::env::set_var("CARGO_PROFILE_RELEASE_OVERFLOW_CHECKS", "false");
        }
        
        Ok(())
    }
    
    /// Run benchmarks
    pub fn bench(&self) -> Result<()> {
        println!("📊 Running benchmarks for TRUNg project: {}", self.config.project.name);
        
        self.generate_cargo_toml()?;
        
        let mut cmd = Command::new("cargo");
        cmd.arg("bench");
        cmd.current_dir(&self.workspace_dir);
        
        println!("📦 Running: {:?}", cmd);
        let output = cmd.output()
            .with_context(|| "Failed to run benchmarks")?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Benchmarks failed: {}", stderr);
        }
        
        println!("✅ Benchmarks completed!");
        Ok(())
    }
    
    /// Format code
    pub fn format(&self) -> Result<()> {
        println!("🎨 Formatting code for TRUNg project: {}", self.config.project.name);
        
        let mut cmd = Command::new("cargo");
        cmd.arg("fmt");
        cmd.current_dir(&self.workspace_dir);
        
        println!("📦 Running: {:?}", cmd);
        let output = cmd.output()
            .with_context(|| "Failed to format code")?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Format failed: {}", stderr);
        }
        
        println!("✅ Code formatted!");
        Ok(())
    }
    
    /// Run linter
    pub fn lint(&self) -> Result<()> {
        println!("🔍 Running linter for TRUNg project: {}", self.config.project.name);
        
        let mut cmd = Command::new("cargo");
        cmd.args(["clippy", "--", "-D", "warnings"]);
        cmd.current_dir(&self.workspace_dir);
        
        println!("📦 Running: {:?}", cmd);
        let output = cmd.output()
            .with_context(|| "Failed to run linter")?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Linting failed: {}", stderr);
        }
        
        println!("✅ No linting issues found!");
        Ok(())
    }
    
    /// Generate documentation
    pub fn doc(&self, open: bool) -> Result<()> {
        println!("📚 Generating documentation for TRUNg project: {}", self.config.project.name);
        
        let mut cmd = Command::new("cargo");
        cmd.arg("doc");
        
        if open {
            cmd.arg("--open");
        }
        
        cmd.current_dir(&self.workspace_dir);
        
        println!("📦 Running: {:?}", cmd);
        let output = cmd.output()
            .with_context(|| "Failed to generate documentation")?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Documentation generation failed: {}", stderr);
        }
        
        println!("✅ Documentation generated!");
        Ok(())
    }
    
    /// Get configuration
    pub fn config(&self) -> &TrungConfig {
        &self.config
    }
    
    /// Show project information
    pub fn info(&self) -> Result<()> {
        println!("📋 TRUNg Project Information:");
        println!("  Name: {}", self.config.project.name);
        println!("  Version: {}", self.config.project.version);
        println!("  Description: {}", self.config.project.description);
        println!("  Authors: {}", self.config.project.authors.join(", "));
        println!("  License: {}", self.config.project.license);
        
        if let Some(repo) = &self.config.project.repository {
            println!("  Repository: {}", repo);
        }
        
        if let Some(homepage) = &self.config.project.homepage {
            println!("  Homepage: {}", homepage);
        }
        
        println!("  Edition: {}", self.config.project.edition);
        println!("  Rust Version: {}", self.config.project.rust_version);
        
        println!("\n🔧 Build Configuration:");
        println!("  Target: {}", self.config.configure.build_target);
        println!("  Optimization Level: {}", self.config.configure.optimization_level);
        println!("  LTO: {}", self.config.configure.lto);
        println!("  Codegen Units: {}", self.config.configure.codegen_units);
        println!("  Panic: {}", self.config.configure.panic);
        
        println!("\n📦 Features:");
        for feature in &self.config.features.default {
            println!("  - {}", feature);
        }
        
        println!("\n🎯 Binaries:");
        for binary in &self.config.bin {
            println!("  - {}: {}", binary.name, binary.path);
        }
        
        Ok(())
    }
}

/// Create TRUNg builder from current directory
pub fn from_current_dir() -> Result<TrungBuilder> {
    TrungBuilder::new(std::env::current_dir()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs;
    
    #[test]
    fn test_trung_builder_creation() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("trunk.toml");
        
        let config = TrungConfig::default();
        config.save_to_file(&config_path).unwrap();
        
        let builder = TrungBuilder::new(temp_dir.path()).unwrap();
        assert_eq!(builder.config.project.name, "trung-project");
    }
    
    #[test]
    fn test_cargo_toml_generation() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("trunk.toml");
        
        let config = TrungConfig::default();
        config.save_to_file(&config_path).unwrap();
        
        let builder = TrungBuilder::new(temp_dir.path()).unwrap();
        builder.generate_cargo_toml().unwrap();
        
        let cargo_toml_path = temp_dir.path().join("Cargo.toml");
        assert!(cargo_toml_path.exists());
        
        let content = fs::read_to_string(&cargo_toml_path).unwrap();
        assert!(content.contains("[package]"));
        assert!(content.contains("trung-project"));
    }
}
