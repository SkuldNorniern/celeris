use super::ast::{Node, BinaryOperator};
use super::value::{JsValue, JsObject};
use std::collections::HashMap;
use std::error::Error;
use std::rc::Rc;
use std::cell::RefCell;
use log::debug;

pub struct Runtime {
    global_scope: Scope,
    call_stack: Vec<Scope>,
}

#[derive(Debug, Clone)]
pub struct Scope {
    variables: HashMap<String, JsValue>,
    parent: Option<Box<Scope>>,
}

impl Scope {
    pub fn new(parent: Option<Box<Scope>>) -> Self {
        Self {
            variables: HashMap::new(),
            parent,
        }
    }
}

impl Runtime {
    pub fn new() -> Self {
        let mut runtime = Self {
            global_scope: Scope::new(None),
            call_stack: Vec::new(),
        };

        // Initialize window object in global scope
        let window = JsValue::Object(Rc::new(RefCell::new(JsObject::new())));
        runtime.set_variable("window", window).expect("Failed to initialize window object");

        runtime
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
                self.evaluate_binary_op(op, &left_val, &right_val)
            }
            
            Node::AssignmentExpr { left, right } => {
                let value = self.evaluate_node(right)?;
                match &**left {
                    Node::Identifier(name) => {
                        self.set_variable(name, value.clone())?;
                        Ok(value)
                    }
                    Node::MemberExpr { object, property, computed } => {
                        let obj = self.evaluate_node(object)?;
                        let prop = if *computed {
                            self.evaluate_node(property)?
                        } else {
                            if let Node::Identifier(name) = &**property {
                                JsValue::String(name.clone())
                            } else {
                                return Err("Invalid property in member expression".into());
                            }
                        };
                        self.set_property(&obj, &prop, value.clone())?;
                        Ok(value)
                    }
                    _ => Err("Invalid assignment target".into()),
                }
            }
            
            Node::ArrayLiteral(elements) => {
                let obj = Rc::new(RefCell::new(JsObject::new()));
                
                // Evaluate each element
                for (i, element) in elements.iter().enumerate() {
                    let value = self.evaluate_node(element)?;
                    obj.borrow_mut().set_property(i.to_string(), value);
                }
                
                // Set length property
                obj.borrow_mut().set_property("length".to_string(), JsValue::Number(elements.len() as f64));
                
                Ok(JsValue::Object(obj))
            }
            
            Node::NewExpr { constructor, arguments } => {
                let constructor_value = self.evaluate_node(constructor)?;
                let mut arg_values = Vec::new();
                
                for arg in arguments {
                    arg_values.push(self.evaluate_node(arg)?);
                }
                
                match constructor_value {
                    JsValue::Function(func) => {
                        // Create a new object with the function's prototype
                        let obj = Rc::new(RefCell::new(JsObject::new()));
                        // TODO: Set up prototype chain
                        
                        // Call the constructor with the new object as 'this'
                        // TODO: Implement proper constructor calling
                        
                        Ok(JsValue::Object(obj))
                    }
                    _ => Err("Constructor must be a function".into()),
                }
            }
            
            Node::MemberExpr { object, property, computed } => {
                debug!(target: "javascript", "Evaluating member expression");
                let obj = self.evaluate_node(object)?;
                debug!(target: "javascript", "Object evaluated to: {:?}", obj);
                let prop = if *computed {
                    self.evaluate_node(property)?
                } else {
                    if let Node::Identifier(name) = &**property {
                        debug!(target: "javascript", "Property name: {}", name);
                        JsValue::String(name.clone())
                    } else {
                        return Err("Invalid property in member expression".into());
                    }
                };
                self.get_property(&obj, &prop)
            }
            
            Node::Identifier(name) => {
                debug!(target: "javascript", "Looking up variable: {}", name);
                if let Some(scope) = self.find_scope_with_variable(name) {
                    Ok(scope.variables.get(name).unwrap().clone())
                } else {
                    debug!(target: "javascript", "Variable not found: {}", name);
                    Ok(JsValue::Undefined)
                }
            }
            
            Node::ObjectLiteral(properties) => {
                debug!(target: "javascript", "Evaluating object literal with {} properties", properties.len());
                let obj = Rc::new(RefCell::new(JsObject::new()));
                
                for (key, value_node) in properties {
                    let value = self.evaluate_node(value_node)?;
                    debug!(target: "javascript", "Setting object property '{}' to {:?}", key, value);
                    obj.borrow_mut().set_property(key.clone(), value);
                }
                
                Ok(JsValue::Object(obj))
            }
            
            _ => Ok(JsValue::Undefined),
        }
    }

    fn evaluate_binary_op(&mut self, op: &BinaryOperator, left: &JsValue, right: &JsValue) -> Result<JsValue, Box<dyn Error>> {
        match op {
            BinaryOperator::Add => {
                match (left, right) {
                    (JsValue::Number(a), JsValue::Number(b)) => Ok(JsValue::Number(a + b)),
                    (JsValue::String(a), JsValue::String(b)) => Ok(JsValue::String(format!("{}{}", a, b))),
                    _ => Err("Invalid operands for addition".into()),
                }
            },
            _ => Ok(JsValue::Undefined),
        }
    }

    fn set_variable(&mut self, name: &str, value: JsValue) -> Result<(), Box<dyn Error>> {
        if let Some(scope) = self.find_scope_with_variable(name) {
            scope.variables.insert(name.to_string(), value);
            Ok(())
        } else {
            if let Some(current_scope) = self.call_stack.last_mut() {
                current_scope.variables.insert(name.to_string(), value);
                Ok(())
            } else {
                self.global_scope.variables.insert(name.to_string(), value);
                Ok(())
            }
        }
    }

    fn find_scope_with_variable(&mut self, name: &str) -> Option<&mut Scope> {
        for scope in self.call_stack.iter_mut().rev() {
            if scope.variables.contains_key(name) {
                return Some(scope);
            }
        }

        if self.global_scope.variables.contains_key(name) {
            return Some(&mut self.global_scope);
        }

        None
    }

    fn set_property(&mut self, obj: &JsValue, prop: &JsValue, value: JsValue) -> Result<(), Box<dyn Error>> {
        debug!(target: "javascript", "Setting property {:?} = {:?} on value {:?}", prop, value, obj);
        match obj {
            JsValue::Object(obj_ref) => {
                let prop_name = match prop {
                    JsValue::String(s) => s.clone(),
                    JsValue::Number(n) => n.to_string(),
                    _ => return Err("Property key must be a string or number".into()),
                };
                
                obj_ref.borrow_mut().set_property(prop_name, value);
                Ok(())
            }
            _ => {
                debug!(target: "javascript", "Failed to set property: value is not an object");
                Err("Cannot set property on non-object value".into())
            }
        }
    }

    fn get_property(&self, obj: &JsValue, prop: &JsValue) -> Result<JsValue, Box<dyn Error>> {
        match obj {
            JsValue::Object(obj_ref) => {
                let prop_name = match prop {
                    JsValue::String(s) => s.to_string(),
                    JsValue::Number(n) => n.to_string(),
                    _ => return Err("Property key must be a string or number".into()),
                };
                
                Ok(obj_ref.borrow().get_property(&prop_name)
                    .cloned()
                    .unwrap_or(JsValue::Undefined))
            }
            _ => Err("Cannot get property of non-object value".into()),
        }
    }
} 