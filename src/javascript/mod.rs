mod ast;
mod interpreter;
mod parser;
mod runtime;
mod value;

use std::error::Error;
use value::JsValue;

pub struct JavaScriptEngine {
    runtime: runtime::Runtime,
}

impl JavaScriptEngine {
    pub fn new() -> Self {
        Self {
            runtime: runtime::Runtime::new(),
        }
    }

    pub fn evaluate(&mut self, script: &str) -> Result<JsValue, Box<dyn Error>> {
        // 1. Parse the script into AST
        let ast = parser::parse(script)?;
        
        // 2. Execute the AST using the runtime
        self.runtime.execute(&ast)
    }
} 