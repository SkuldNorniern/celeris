use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;
use super::ast::Node;
use super::runtime::Scope;

#[derive(Debug, Clone)]
pub enum JsValue {
    Undefined,
    Null,
    Boolean(bool),
    Number(f64),
    String(String),
    Object(Rc<RefCell<JsObject>>),
    Function(JsFunction),
}

#[derive(Debug, Clone)]
pub struct JsObject {
    properties: HashMap<String, JsValue>,
    prototype: Option<Box<JsObject>>,
}

impl JsObject {
    pub fn new() -> Self {
        Self {
            properties: HashMap::new(),
            prototype: None,
        }
    }

    pub fn set_property(&mut self, name: String, value: JsValue) {
        self.properties.insert(name, value);
    }

    pub fn get_property(&self, name: &str) -> Option<&JsValue> {
        self.properties.get(name).or_else(|| {
            self.prototype
                .as_ref()
                .and_then(|proto| proto.get_property(name))
        })
    }

    pub fn is_array(&self) -> bool {
        self.properties.contains_key("length") && 
        self.properties.iter().any(|(k, _)| k.parse::<usize>().is_ok())
    }

    pub fn get_length(&self) -> Option<usize> {
        if let Some(JsValue::Number(n)) = self.get_property("length") {
            Some(*n as usize)
        } else {
            None
        }
    }

    pub fn get_element(&self, index: usize) -> Option<&JsValue> {
        self.get_property(&index.to_string())
    }

    pub fn set_element(&mut self, index: usize, value: JsValue) {
        self.set_property(index.to_string(), value);
        if let Some(JsValue::Number(len)) = self.get_property("length") {
            if index >= *len as usize {
                self.set_property("length".to_string(), JsValue::Number((index + 1) as f64));
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct JsFunction {
    name: Option<String>,
    params: Vec<String>,
    body: Vec<Node>,
    closure: Scope,
} 