mod ast;
mod parser;
mod runtime;
mod value;
mod dom_bridge;
mod tokenizer;

use std::error::Error;
use std::rc::Rc;
use std::cell::RefCell;
use value::JsValue;
use dom_bridge::DomBridge;
use crate::dom::Node as DomNode;

pub struct JavaScriptEngine {
    runtime: runtime::Runtime,
    dom_bridge: DomBridge,
}

impl JavaScriptEngine {
    pub fn new() -> Self {
        Self {
            runtime: runtime::Runtime::new(),
            dom_bridge: DomBridge::new(),
        }
    }

    pub fn evaluate(&mut self, script: &str) -> Result<JsValue, Box<dyn Error>> {
        // Parse the script into AST
        let ast = parser::parse(script)?;
        
        // Execute the AST using the runtime
        self.runtime.execute(&ast)
    }

    pub fn bind_dom(&mut self, dom: &crate::dom::Node) -> Result<(), Box<dyn Error>> {
        self.runtime.bind_dom(dom);
        self.dom_bridge.bind_dom(dom)
    }
    
    pub fn bind_dom_shared(&mut self, dom: Rc<RefCell<DomNode>>) {
        self.runtime.bind_dom_shared(dom);
    }

    pub fn handle_event(&mut self, event_name: &str, target: &crate::dom::Node) -> Result<(), Box<dyn Error>> {
        self.dom_bridge.handle_event(event_name, target)
    }
    
    pub fn runtime_mut(&mut self) -> &mut runtime::Runtime {
        &mut self.runtime
    }
} 