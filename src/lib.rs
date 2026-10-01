//! tru_id_core — TRU Language Library
//!
//! Crate name  : tru_id_core
//! Extension   : .tru  (.run)
//! Language ID : tru_id
//!
//! ## Components
//!
//! | Module             | Purpose                                          |
//! |--------------------|--------------------------------------------------|
//! | `tru_lang`         | Full language pipeline (lexer → parser → interp) |
//! | `trunk_lang_core`  | High-performance runtime primitives              |
//! | `trung_config`     | trunk.toml configuration parser                 |
//! | `trung_build`      | Build system (replaces Cargo.toml workflow)      |
//! | `utils`            | Shared helpers, security, time, UUID             |
//! | `infrastructure`   | Artifactory, BurnLM integration                  |
//! | `domain`           | User, auth, service response models              |

use std::collections::HashMap;
use std::sync::Arc;
use warp::Rejection;

// Core modules
pub mod utils;
pub mod infrastructure;
pub mod domain;
pub mod secure;
pub mod aimodels;
pub mod models;

// High-performance core module
pub mod trunk_lang_core;

// TRU Language — lexer / parser / interpreter / compiler
// tru_id_* crate naming convention
pub mod tru_lang;

// TRUNg Configuration Parser
pub mod trung_config;

// TRUNg Build System
pub mod trung_build;

// Type aliases for convenience
pub type WebResult<T> = std::result::Result<T, Rejection>;
pub type Users = Arc<HashMap<String, User>>;

// Re-export commonly used items
pub use utils::sharedUtils::*;
pub use utils::helperUtils::*;
pub use utils::secureUtils::*;

// Re-export optimized versions
pub use utils::secureUtils_optimized::*;
pub use utils::helperUtils_optimized::*;

pub use infrastructure::upload_artifactory;
pub use infrastructure::copy_to_burnlm;

pub use domain::models::routine::user::{User, UserDB};
pub use domain::models::routine::login::{LoginRequest, LoginResponse};
pub use domain::models::routine::srv::RespSrv;

// Re-export trunk lang core
pub use trunk_lang_core::*;

// tru_id language public surface
pub use tru_lang::{run_source, run_file, transpile_source, transpile_file,
                   transpile_to_rs, TruRepl, TruError, TruResult,
                   Parser, Interpreter, Compiler, Value, Env,
                   compile_tru, TRU_VERSION, TRU_EXTENSION, TRU_CRATE_NAME};

// Re-export TRUNg configuration
pub use trung_config::*;

// Re-export TRUNg build system
pub use trung_build::*;

/// Prelude module for common imports
pub mod prelude {
    pub use crate::utils::*;
    pub use crate::infrastructure::*;
    pub use crate::domain::models::routine::*;
    pub use crate::{WebResult, Users};
    
    // Include optimized versions
    pub use crate::utils::secureUtils_optimized::*;
    pub use crate::utils::helperUtils_optimized::*;
    
    // Include trunk lang core
    pub use crate::trunk_lang_core::*;
}

/// Performance-optimized prelude for high-performance applications
pub mod performance_prelude {
    pub use crate::utils::secureUtils_optimized::*;
    pub use crate::utils::helperUtils_optimized::*;
    pub use crate::trunk_lang_core::*;
    pub use crate::tru_lang::*;
    pub use crate::prelude::*;
}

/// tru_id prelude — everything needed to write and run TRU programs
pub mod tru_prelude {
    pub use crate::tru_lang::{
        run_source, run_file, transpile_source, transpile_file, transpile_to_rs,
        TruRepl, TruError, TruResult,
        Parser, Interpreter, Compiler, Value, Env,
        compile_tru, TRU_VERSION, TRU_EXTENSION, TRU_CRATE_NAME,
        lexer::{Lexer, Token, TokenKind},
        ast::{Program, Stmt, Expr},
    };
    pub use crate::trunk_lang_core::*;
    pub use crate::trung_config::*;
}

/// TRUNg prelude for configuration and build system
pub mod trung_prelude {
    pub use crate::trung_config::*;
    pub use crate::trung_build::*;
    pub use crate::tru_prelude::*;
}

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Library name
pub const LIB_NAME: &str = env!("CARGO_PKG_NAME");

/// Library description
pub const LIB_DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_library_info() {
        assert_eq!(LIB_NAME, "tru_id_core");
        assert!(!VERSION.is_empty());
        assert!(!LIB_DESCRIPTION.is_empty());
    }

    #[test]
    fn test_utility_functions() {
        let time = utils::helperUtils::current_time();
        assert!(!time.is_empty());

        let uuid = utils::helperUtils::uniqueIdUUID();
        assert!(!uuid.is_empty());
    }
}
