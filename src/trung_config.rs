//! TRUNg Configuration Parser
//! 
//! Reads and parses trunk.toml configuration files instead of Cargo.toml
//! Provides enhanced configuration options for TRU language and performance optimizations

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::fs;
use anyhow::{Result, Context};
use toml;

/// TRUNg Configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrungConfig {
    pub project: ProjectConfig,
    pub configure: ConfigureConfig,
    pub bin: Vec<BinaryConfig>,
    pub src: SourceConfig,
    pub dependencies: DependencyConfig,
    pub dev_dependencies: DependencyConfig,
    pub features: FeatureConfig,
    pub build: BuildConfig,
    pub profiles: ProfileConfig,
    pub sgd_fsdp: SgdFsdpConfig,
    pub installer: InstallerConfig,
    pub embedded: EmbeddedConfig,
    pub ai_ml: AiMlConfig,
    pub scripts: ScriptsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub name: String,
    pub version: String,
    pub description: String,
    pub authors: Vec<String>,
    pub license: String,
    pub repository: Option<String>,
    pub homepage: Option<String>,
    pub keywords: Vec<String>,
    pub categories: Vec<String>,
    pub edition: String,
    pub rust_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigureConfig {
    pub build_target: String,
    pub optimization_level: u8,
    pub lto: bool,
    pub codegen_units: u8,
    pub panic: String,
    pub overflow_checks: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryConfig {
    pub name: String,
    pub path: String,
    pub target: Option<String>,
    pub wasm_target: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceConfig {
    pub main: String,
    pub root: String,
    pub include: Vec<String>,
    pub exclude: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyConfig {
    pub dependencies: HashMap<String, toml::Value>,
    #[serde(default)]
    pub target_dependencies: HashMap<String, HashMap<String, toml::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureConfig {
    pub default: Vec<String>,
    #[serde(default)]
    pub features: HashMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildConfig {
    pub rustflags: Vec<String>,
    #[serde(default)]
    pub target_rustflags: HashMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileConfig {
    pub dev: ProfileDetails,
    pub release: ProfileDetails,
    pub bench: ProfileDetails,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileDetails {
    pub opt_level: u8,
    pub debug: bool,
    pub lto: bool,
    pub codegen_units: u32,
    pub panic: Option<String>,
    pub overflow_checks: bool,
    pub strip: Option<bool>,
    pub incremental: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SgdFsdpConfig {
    pub enabled: bool,
    pub sharded_training: bool,
    pub fully_sharded: bool,
    pub optimizer: String,
    pub learning_rate: f64,
    pub batch_size: u32,
    pub gradient_accumulation_steps: u32,
    pub model: SgdFsdpModelConfig,
    pub training: SgdFsdpTrainingConfig,
    pub hardware: SgdFsdpHardwareConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SgdFsdpModelConfig {
    pub model_type: String,
    pub hidden_size: usize,
    pub num_layers: usize,
    pub num_attention_heads: usize,
    pub max_sequence_length: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SgdFsdpTrainingConfig {
    pub epochs: usize,
    pub warmup_steps: usize,
    pub weight_decay: f64,
    pub adam_beta1: f64,
    pub adam_beta2: f64,
    pub adam_epsilon: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SgdFsdpHardwareConfig {
    pub gpu_count: usize,
    pub memory_per_gpu: String,
    pub mixed_precision: bool,
    pub gradient_checkpointing: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallerConfig {
    pub name: String,
    pub version: String,
    pub targets: Vec<String>,
    pub binary_name: String,
    pub install_path: String,
    pub dioxus: InstallerDioxusConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallerDioxusConfig {
    pub enabled: bool,
    pub gui: bool,
    pub auto_update: bool,
    pub telemetry: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddedConfig {
    pub framework: String,
    pub wasm: bool,
    pub memory_limit: String,
    pub stack_size: String,
    pub heap_size: String,
    pub coin: EmbeddedCoinConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddedCoinConfig {
    pub enabled: bool,
    pub algorithm: String,
    pub difficulty: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiMlConfig {
    pub frameworks: Vec<String>,
    pub models: Vec<String>,
    pub optimizers: Vec<String>,
    pub schedulers: Vec<String>,
    pub wasm: AiMlWasmConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiMlWasmConfig {
    pub enabled: bool,
    pub backend: String,
    pub precision: String,
    pub batch_size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptsConfig {
    pub build: String,
    pub build_wasm: String,
    pub test: String,
    pub bench: String,
    pub lint: String,
    pub format: String,
    pub doc: String,
    pub compile_tru: String,
    pub run_tru: String,
    pub install: String,
    pub clean: String,
}

impl Default for TrungConfig {
    fn default() -> Self {
        Self {
            project: ProjectConfig {
                name: "trung-project".to_string(),
                version: "0.1.0".to_string(),
                description: "A TRUNg project".to_string(),
                authors: vec!["author@example.com".to_string()],
                license: "MIT".to_string(),
                repository: None,
                homepage: None,
                keywords: vec![],
                categories: vec![],
                edition: "2021".to_string(),
                rust_version: "1.75".to_string(),
            },
            configure: ConfigureConfig {
                build_target: "release".to_string(),
                optimization_level: 3,
                lto: true,
                codegen_units: 1,
                panic: "abort".to_string(),
                overflow_checks: false,
            },
            bin: vec![],
            src: SourceConfig {
                main: "src/lib.rs".to_string(),
                root: "src".to_string(),
                include: vec!["**/*.rs".to_string()],
                exclude: vec![],
            },
            dependencies: DependencyConfig {
                dependencies: HashMap::new(),
                target_dependencies: HashMap::new(),
            },
            dev_dependencies: DependencyConfig {
                dependencies: HashMap::new(),
                target_dependencies: HashMap::new(),
            },
            features: FeatureConfig {
                default: vec!["std".to_string()],
                features: HashMap::new(),
            },
            build: BuildConfig {
                rustflags: vec![],
                target_rustflags: HashMap::new(),
            },
            profiles: ProfileConfig {
                dev: ProfileDetails {
                    opt_level: 1,
                    debug: true,
                    lto: false,
                    codegen_units: 256,
                    panic: Some("unwind".to_string()),
                    overflow_checks: true,
                    strip: Some(false),
                    incremental: Some(true),
                },
                release: ProfileDetails {
                    opt_level: 3,
                    debug: false,
                    lto: true,
                    codegen_units: 1,
                    panic: Some("abort".to_string()),
                    overflow_checks: false,
                    strip: Some(true),
                    incremental: Some(false),
                },
                bench: ProfileDetails {
                    opt_level: 3,
                    debug: false,
                    lto: true,
                    codegen_units: 1,
                    panic: Some("abort".to_string()),
                    overflow_checks: false,
                    strip: Some(true),
                    incremental: Some(false),
                },
            },
            sgd_fsdp: SgdFsdpConfig {
                enabled: false,
                sharded_training: false,
                fully_sharded: false,
                optimizer: "sgd".to_string(),
                learning_rate: 0.001,
                batch_size: 32,
                gradient_accumulation_steps: 4,
                model: SgdFsdpModelConfig {
                    model_type: "transformer".to_string(),
                    hidden_size: 768,
                    num_layers: 12,
                    num_attention_heads: 12,
                    max_sequence_length: 512,
                },
                training: SgdFsdpTrainingConfig {
                    epochs: 100,
                    warmup_steps: 1000,
                    weight_decay: 0.01,
                    adam_beta1: 0.9,
                    adam_beta2: 0.999,
                    adam_epsilon: 1e-8,
                },
                hardware: SgdFsdpHardwareConfig {
                    gpu_count: 1,
                    memory_per_gpu: "8GB".to_string(),
                    mixed_precision: false,
                    gradient_checkpointing: false,
                },
            },
            installer: InstallerConfig {
                name: "trung".to_string(),
                version: "0.1.0".to_string(),
                targets: vec!["x86_64-linux".to_string()],
                binary_name: "trung".to_string(),
                install_path: "/usr/local/bin".to_string(),
                dioxus: InstallerDioxusConfig {
                    enabled: false,
                    gui: false,
                    auto_update: false,
                    telemetry: false,
                },
            },
            embedded: EmbeddedConfig {
                framework: "slint".to_string(),
                wasm: false,
                memory_limit: "64MB".to_string(),
                stack_size: "1MB".to_string(),
                heap_size: "32MB".to_string(),
                coin: EmbeddedCoinConfig {
                    enabled: false,
                    algorithm: "sha256".to_string(),
                    difficulty: 1,
                },
            },
            ai_ml: AiMlConfig {
                frameworks: vec!["burn".to_string()],
                models: vec!["transformer".to_string()],
                optimizers: vec!["sgd".to_string()],
                schedulers: vec!["cosine".to_string()],
                wasm: AiMlWasmConfig {
                    enabled: false,
                    backend: "webgpu".to_string(),
                    precision: "f32".to_string(),
                    batch_size: 1,
                },
            },
            scripts: ScriptsConfig {
                build: "cargo build --release".to_string(),
                build_wasm: "wasm-pack build --target web".to_string(),
                test: "cargo test".to_string(),
                bench: "cargo bench".to_string(),
                lint: "cargo clippy".to_string(),
                format: "cargo fmt".to_string(),
                doc: "cargo doc --open".to_string(),
                compile_tru: "cargo run --bin tru-compiler --".to_string(),
                run_tru: "cargo run --bin trunk --".to_string(),
                install: "cargo install --path .".to_string(),
                clean: "cargo clean".to_string(),
            },
        }
    }
}

impl TrungConfig {
    /// Load configuration from trunk.toml file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read trunk.toml from {:?}", path.as_ref()))?;
        
        let config: TrungConfig = toml::from_str(&content)
            .with_context(|| "Failed to parse trunk.toml")?;
        
        Ok(config)
    }
    
    /// Load configuration from current directory
    pub fn load() -> Result<Self> {
        Self::load_from_file("trunk.toml")
    }
    
    /// Save configuration to trunk.toml file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = toml::to_string_pretty(self)
            .with_context(|| "Failed to serialize configuration")?;
        
        fs::write(&path, content)
            .with_context(|| format!("Failed to write trunk.toml to {:?}", path.as_ref()))?;
        
        Ok(())
    }
    
    /// Get optimization flags based on configuration
    pub fn get_optimization_flags(&self) -> Vec<String> {
        let mut flags = vec![];
        
        if self.configure.lto {
            flags.push("-C".to_string());
            flags.push("link-time-opt".to_string());
        }
        
        flags.push("-C".to_string());
        flags.push(format!("opt-level={}", self.configure.optimization_level));
        
        flags.push("-C".to_string());
        flags.push(format!("codegen-units={}", self.configure.codegen_units));
        
        if self.configure.panic == "abort" {
            flags.push("-C".to_string());
            flags.push("panic=abort".to_string());
        }
        
        // Add custom rustflags
        flags.extend(self.build.rustflags.clone());
        
        flags
    }
    
    /// Get target-specific optimization flags
    pub fn get_target_flags(&self, target: &str) -> Vec<String> {
        self.build.target_rustflags
            .get(target)
            .cloned()
            .unwrap_or_default()
    }
    
    /// Check if a feature is enabled
    pub fn is_feature_enabled(&self, feature: &str) -> bool {
        self.features.default.contains(&feature.to_string())
    }
    
    /// Get dependency version
    pub fn get_dependency_version(&self, dep: &str) -> Option<&toml::Value> {
        self.dependencies.dependencies.get(dep)
    }
    
    /// Generate Cargo.toml equivalent for compatibility
    pub fn generate_cargo_toml(&self) -> Result<String> {
        let mut cargo_toml = toml::map::Map::new();
        
        // Package section
        let mut package = toml::map::Map::new();
        package.insert("name".to_string(), toml::Value::String(self.project.name.clone()));
        package.insert("version".to_string(), toml::Value::String(self.project.version.clone()));
        package.insert("edition".to_string(), toml::Value::String(self.project.edition.clone()));
        package.insert("description".to_string(), toml::Value::String(self.project.description.clone()));
        package.insert("license".to_string(), toml::Value::String(self.project.license.clone()));
        package.insert("authors".to_string(), toml::Value::Array(
            self.project.authors.iter().cloned().map(toml::Value::String).collect()
        ));
        
        if let Some(repo) = &self.project.repository {
            package.insert("repository".to_string(), toml::Value::String(repo.clone()));
        }
        
        cargo_toml.insert("package".to_string(), toml::Value::Table(package));
        
        // Dependencies section
        if !self.dependencies.dependencies.is_empty() {
            cargo_toml.insert("dependencies".to_string(), toml::Value::Table(
                self.dependencies.dependencies.iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect()
            ));
        }
        
        // Features section
        let mut features_map = toml::map::Map::new();
        features_map.insert("default".to_string(), toml::Value::Array(
            self.features.default.iter().cloned().map(toml::Value::String).collect()
        ));
        
        for (feature, deps) in &self.features.features {
            features_map.insert(feature.clone(), toml::Value::Array(
                deps.iter().cloned().map(toml::Value::String).collect()
            ));
        }
        
        cargo_toml.insert("features".to_string(), toml::Value::Table(features_map));
        
        // Profiles section
        let mut profiles = toml::map::Map::new();
        
        let mut dev_profile = toml::map::Map::new();
        dev_profile.insert("opt-level".to_string(), toml::Value::Integer(self.profiles.dev.opt_level.into()));
        dev_profile.insert("debug".to_string(), toml::Value::Boolean(self.profiles.dev.debug));
        dev_profile.insert("lto".to_string(), toml::Value::Boolean(self.profiles.dev.lto));
        dev_profile.insert("codegen-units".to_string(), toml::Value::Integer(self.profiles.dev.codegen_units.into()));
        profiles.insert("dev".to_string(), toml::Value::Table(dev_profile));
        
        let mut release_profile = toml::map::Map::new();
        release_profile.insert("opt-level".to_string(), toml::Value::Integer(self.profiles.release.opt_level.into()));
        release_profile.insert("debug".to_string(), toml::Value::Boolean(self.profiles.release.debug));
        release_profile.insert("lto".to_string(), toml::Value::Boolean(self.profiles.release.lto));
        release_profile.insert("codegen-units".to_string(), toml::Value::Integer(self.profiles.release.codegen_units.into()));
        profiles.insert("release".to_string(), toml::Value::Table(release_profile));
        
        cargo_toml.insert("profile".to_string(), toml::Value::Table(profiles));
        
        Ok(toml::to_string_pretty(&cargo_toml)?)
    }
}

/// Global configuration instance
static TRUNG_CONFIG: std::sync::OnceLock<TrungConfig> = std::sync::OnceLock::new();

/// Get global TRUNg configuration
pub fn get_trung_config() -> &'static TrungConfig {
    TRUNG_CONFIG.get_or_init(|| {
        TrungConfig::load().unwrap_or_else(|e| {
            eprintln!("Warning: Failed to load trunk.toml: {}", e);
            eprintln!("Using default configuration");
            TrungConfig::default()
        })
    })
}

/// Initialize TRUNg configuration from file
pub fn init_config_from_file<P: AsRef<Path>>(path: P) -> Result<()> {
    let config = TrungConfig::load_from_file(path)?;
    TRUNG_CONFIG.set(config).map_err(|_| anyhow::anyhow!("Configuration already initialized"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;
    
    #[test]
    fn test_config_parsing() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("trunk.toml");
        
        let config = TrungConfig::default();
        config.save_to_file(&config_path).unwrap();
        
        let loaded_config = TrungConfig::load_from_file(&config_path).unwrap();
        assert_eq!(config.project.name, loaded_config.project.name);
        assert_eq!(config.project.version, loaded_config.project.version);
    }
    
    #[test]
    fn test_optimization_flags() {
        let config = TrungConfig::default();
        let flags = config.get_optimization_flags();
        
        assert!(flags.iter().any(|f| f.contains("opt-level")));
        assert!(flags.iter().any(|f| f.contains("codegen-units")));
    }
    
    #[test]
    fn test_cargo_toml_generation() {
        let config = TrungConfig::default();
        let cargo_toml = config.generate_cargo_toml().unwrap();

        assert!(cargo_toml.contains("[package]"));
        assert!(cargo_toml.contains("name = \"trung-project\""));
        // `profile` only holds sub-tables, so toml emits no bare [profile] header.
        // Same for `dependencies`: omitted entirely when the config has none.
        assert!(cargo_toml.contains("[profile.release]"));
        assert!(!cargo_toml.contains("[dependencies]"));
    }
}
