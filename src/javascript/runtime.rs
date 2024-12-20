use super::ast::Node;
use super::value::JsValue;
use std::collections::HashMap;
use std::error::Error;

pub struct Runtime {
    global_scope: Scope,
    call_stack: Vec<Scope>,
}

struct Scope {
    variables: HashMap<String, JsValue>,
    parent: Option<Box<Scope>>,
}

impl Runtime {
    pub fn new() -> Self {
        Self {
            global_scope: Scope::new(None),
            call_stack: Vec::new(),
        }
    }

    pub fn execute(&mut self, ast: &Node) -> Result<JsValue, Box<dyn Error>> {
        match ast {
            Node::Program(statements) => {
                let mut result = JsValue::Undefined;
                for stmt in statements {
                    result = self.evaluate_node(stmt)?;
                }
                Ok(result)
            }
            _ => self.evaluate_node(ast),
        }
    }

    fn evaluate_node(&mut self, node: &Node) -> Result<JsValue, Box<dyn Error>> {
        match node {
            Node::Number(n) => Ok(JsValue::Number(*n)),
            Node::String(s) => Ok(JsValue::String(s.clone())),
            Node::Boolean(b) => Ok(JsValue::Boolean(*b)),
            Node::Null => Ok(JsValue::Null),
            Node::Undefined => Ok(JsValue::Undefined),
            
            Node::BinaryOp { op, left, right } => {
                let left_val = self.evaluate_node(left)?;
                let right_val = self.evaluate_node(right)?;
                self.evaluate_binary_op(op, left_val, right_val)
            }
            
            // Add more evaluation cases...
            _ => Ok(JsValue::Undefined),
        }
    }
} 