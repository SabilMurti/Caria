//! Caria Embedding API - Embeddable Scripting Runtime
//! 
//! This module provides a simple API for host applications to embed Caria
//! scripting capabilities. Ideal for AI agents, automation tools, and extension systems.
//!
//! # Example Usage
//! ```rust
//! let mut engine = caria_runtime::CariaEngine::new()?;
//! let script = r#"
//!     rune main() -> int {
//!         say("Hello from embedded Caria!")
//!         ret 42
//!     }
//! "#;
//! let result = engine.eval(script)?;
//! println!("Result: {}", result);
//! # Ok::<_, String>(())
//! ```

// Note: Circular dependency prevented - caria_codegen_native depends on caria_runtime
// This module provides a minimal stub API until the architecture is refactored
use std::fmt;

/// Error type for embedding operations
#[derive(Debug)]
pub enum CariaError {
    Parse(String),
    Compile(String),
    Execution(String),
}

impl std::fmt::Display for CariaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CariaError::Parse(msg) => write!(f, "Parse error: {}", msg),
            CariaError::Compile(msg) => write!(f, "Compile error: {}", msg),
            CariaError::Execution(msg) => write!(f, "Execution error: {}", msg),
        }
    }
}

impl std::error::Error for CariaError {}

/// Compiles and executes Caria scripts (stub implementation)
pub struct CariaEngine {
    _placeholder: (),
}

impl CariaEngine {
    /// Create a new CariaEngine instance
    pub fn new() -> Result<Self, CariaError> {
        Ok(Self { _placeholder: () })
    }
    
    /// Evaluate a Caria script string and return exit code
    /// 
    /// Note: This is a stub implementation pending architecture refactor
    pub fn eval(&mut self, _script: &str) -> Result<i32, CariaError> {
        Err(CariaError::Compile(
            "Embedding API not yet implemented - circular dependency blocker".to_string()
        ))
    }
    
    /// Evaluate a Caria expression (stub)
    pub fn eval_expr(&mut self, _expr_str: &str) -> Result<String, CariaError> {
        Err(CariaError::Compile(
            "Embedding API not yet implemented - circular dependency blocker".to_string()
        ))
    }
}

impl Default for CariaEngine {
    fn default() -> Self {
        Self::new().expect("Failed to create CariaEngine")
    }
}

// Re-export common types
pub mod prelude {
    pub use super::CariaEngine;
    pub use super::CariaError;
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_hello_world() {
        let mut engine = CariaEngine::default();
        let script = r#"
            rune main() -> int {
                say("Hello World")
                ret 42
            }
        "#;
        
        let result = engine.eval(script).unwrap();
        assert_eq!(result, 42);
    }
    
    #[test]
    fn test_arithmetic() {
        let mut engine = CariaEngine::default();
        let script = r#"
            rune main() -> int {
                x := 5
                y := 10
                result := x + y
                say(result)
                ret 0
            }
        "#;
        
        let result = engine.eval(script).unwrap();
        assert_eq!(result, 0);
    }
    
    #[test]
    fn test_control_flow() {
        let mut engine = CariaEngine::default();
        let script = r#"
            rune main() -> int {
                sum := 0
                i := 0
                while (i < 5) {
                    sum := sum + i
                    i := i + 1
                }
                say(sum)
                ret 0
            }
        "#;
        
        let result = engine.eval(script).unwrap();
        assert_eq!(result, 0);
    }
}
