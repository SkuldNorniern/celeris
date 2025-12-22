use super::ast::{Node, BinaryOperator, UnaryOperator};
use super::value::{JsValue, JsObject, JsUserFunction};
use crate::dom::Node as DomNode;
use std::collections::HashMap;
use std::error::Error;
use std::rc::Rc;
use std::cell::RefCell;
use log::debug;

pub struct Runtime {
    global_scope: Scope,
    call_stack: Vec<Scope>,
    dom_root: Option<Rc<RefCell<DomNode>>>, // Store DOM root for DOM operations
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
            dom_root: None,
        };

        // Initialize window object in global scope with common methods
        runtime.init_window();

        // Initialize console object with log, warn, error methods
        runtime.init_console();

        runtime
    }
    
    pub fn bind_dom(&mut self, dom_root: &DomNode) {
        // Store a reference to the DOM root for DOM operations
        // We wrap it in Rc<RefCell<>> to allow shared mutable access
        // Note: This creates a clone of the DOM node, but we'll work with it
        self.dom_root = Some(Rc::new(RefCell::new(dom_root.clone())));
        debug!(target: "javascript", "DOM bound to JavaScript runtime");
    }

    fn init_window(&mut self) {
        let mut window_obj = JsObject::new();
        
        // Common window methods
        window_obj.set("setTimeout", JsValue::NativeFunction("window.setTimeout".to_string()));
        window_obj.set("setInterval", JsValue::NativeFunction("window.setInterval".to_string()));
        window_obj.set("clearTimeout", JsValue::NativeFunction("window.clearTimeout".to_string()));
        window_obj.set("clearInterval", JsValue::NativeFunction("window.clearInterval".to_string()));
        window_obj.set("addEventListener", JsValue::NativeFunction("window.addEventListener".to_string()));
        window_obj.set("removeEventListener", JsValue::NativeFunction("window.removeEventListener".to_string()));
        window_obj.set("alert", JsValue::NativeFunction("window.alert".to_string()));
        window_obj.set("confirm", JsValue::NativeFunction("window.confirm".to_string()));
        window_obj.set("prompt", JsValue::NativeFunction("window.prompt".to_string()));
        window_obj.set("requestAnimationFrame", JsValue::NativeFunction("window.requestAnimationFrame".to_string()));
        window_obj.set("cancelAnimationFrame", JsValue::NativeFunction("window.cancelAnimationFrame".to_string()));
        window_obj.set("getComputedStyle", JsValue::NativeFunction("window.getComputedStyle".to_string()));
        window_obj.set("matchMedia", JsValue::NativeFunction("window.matchMedia".to_string()));
        
        // Window properties
        window_obj.set("innerWidth", JsValue::Number(1920.0));
        window_obj.set("innerHeight", JsValue::Number(1080.0));
        window_obj.set("outerWidth", JsValue::Number(1920.0));
        window_obj.set("outerHeight", JsValue::Number(1080.0));
        window_obj.set("devicePixelRatio", JsValue::Number(1.0));
        
        let window = JsValue::Object(Rc::new(RefCell::new(window_obj)));
        self.set_variable("window", window).expect("Failed to initialize window object");
        
        // Also make window methods available globally (like in browsers)
        self.set_variable("setTimeout", JsValue::NativeFunction("window.setTimeout".to_string())).ok();
        self.set_variable("setInterval", JsValue::NativeFunction("window.setInterval".to_string())).ok();
        self.set_variable("clearTimeout", JsValue::NativeFunction("window.clearTimeout".to_string())).ok();
        self.set_variable("clearInterval", JsValue::NativeFunction("window.clearInterval".to_string())).ok();
        self.set_variable("alert", JsValue::NativeFunction("window.alert".to_string())).ok();
        self.set_variable("requestAnimationFrame", JsValue::NativeFunction("window.requestAnimationFrame".to_string())).ok();
    }
    
    fn init_console(&mut self) {
        let mut console_obj = JsObject::new();
        
        // Add console.log as a native function
        console_obj.set("log", JsValue::NativeFunction("console.log".to_string()));
        console_obj.set("warn", JsValue::NativeFunction("console.warn".to_string()));
        console_obj.set("error", JsValue::NativeFunction("console.error".to_string()));
        console_obj.set("info", JsValue::NativeFunction("console.info".to_string()));
        console_obj.set("debug", JsValue::NativeFunction("console.debug".to_string()));
        
        let console = JsValue::Object(Rc::new(RefCell::new(console_obj)));
        self.set_variable("console", console).expect("Failed to initialize console object");
        
        // Initialize JSON object
        self.init_json();
        
        // Initialize document object (basic stub)
        self.init_document();
        
        // Initialize navigator object
        self.init_navigator();
        
        // Initialize location object
        self.init_location();
    }
    
    fn init_json(&mut self) {
        let mut json_obj = JsObject::new();
        json_obj.set("parse", JsValue::NativeFunction("JSON.parse".to_string()));
        json_obj.set("stringify", JsValue::NativeFunction("JSON.stringify".to_string()));
        let json = JsValue::Object(Rc::new(RefCell::new(json_obj)));
        self.set_variable("JSON", json).expect("Failed to initialize JSON object");
    }
    
    fn init_document(&mut self) {
        let mut doc_obj = JsObject::new();
        
        // Basic document properties
        doc_obj.set("readyState", JsValue::String("complete".to_string()));
        doc_obj.set("title", JsValue::String("".to_string()));
        doc_obj.set("cookie", JsValue::String("".to_string()));
        
        // Basic document methods as native functions
        doc_obj.set("getElementById", JsValue::NativeFunction("document.getElementById".to_string()));
        doc_obj.set("querySelector", JsValue::NativeFunction("document.querySelector".to_string()));
        doc_obj.set("querySelectorAll", JsValue::NativeFunction("document.querySelectorAll".to_string()));
        doc_obj.set("createElement", JsValue::NativeFunction("document.createElement".to_string()));
        doc_obj.set("getElementsByTagName", JsValue::NativeFunction("document.getElementsByTagName".to_string()));
        doc_obj.set("getElementsByClassName", JsValue::NativeFunction("document.getElementsByClassName".to_string()));
        doc_obj.set("addEventListener", JsValue::NativeFunction("document.addEventListener".to_string()));
        doc_obj.set("removeEventListener", JsValue::NativeFunction("document.removeEventListener".to_string()));
        
        let document = JsValue::Object(Rc::new(RefCell::new(doc_obj)));
        self.set_variable("document", document).expect("Failed to initialize document object");
    }
    
    fn init_navigator(&mut self) {
        let mut nav_obj = JsObject::new();
        
        // User agent string
        nav_obj.set("userAgent", JsValue::String("Celeris/0.1 (X11; Linux x86_64)".to_string()));
        nav_obj.set("platform", JsValue::String(std::env::consts::OS.to_string()));
        nav_obj.set("language", JsValue::String("en-US".to_string()));
        nav_obj.set("languages", JsValue::Object(Rc::new(RefCell::new({
            let mut arr = JsObject::new();
            arr.set("0", JsValue::String("en-US".to_string()));
            arr.set("length", JsValue::Number(1.0));
            arr
        }))));
        nav_obj.set("cookieEnabled", JsValue::Boolean(true));
        nav_obj.set("onLine", JsValue::Boolean(true));
        
        let navigator = JsValue::Object(Rc::new(RefCell::new(nav_obj)));
        self.set_variable("navigator", navigator).expect("Failed to initialize navigator object");
    }
    
    fn init_location(&mut self) {
        let mut loc_obj = JsObject::new();
        
        // Basic location properties (will be updated when loading a page)
        loc_obj.set("href", JsValue::String("about:blank".to_string()));
        loc_obj.set("protocol", JsValue::String("https:".to_string()));
        loc_obj.set("host", JsValue::String("".to_string()));
        loc_obj.set("hostname", JsValue::String("".to_string()));
        loc_obj.set("port", JsValue::String("".to_string()));
        loc_obj.set("pathname", JsValue::String("/".to_string()));
        loc_obj.set("search", JsValue::String("".to_string()));
        loc_obj.set("hash", JsValue::String("".to_string()));
        loc_obj.set("origin", JsValue::String("".to_string()));
        
        let location = JsValue::Object(Rc::new(RefCell::new(loc_obj)));
        self.set_variable("location", location).expect("Failed to initialize location object");
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
            
            Node::UnaryOp { op, operand } => {
                match op {
                    UnaryOperator::PostIncrement | UnaryOperator::PostDecrement => {
                        // Postfix: evaluate operand, get value, increment/decrement, return old value
                        let old_val = self.evaluate_node(operand)?;
                        let new_val = match op {
                            UnaryOperator::PostIncrement => {
                                match old_val {
                                    JsValue::Number(n) => JsValue::Number(n + 1.0),
                                    _ => JsValue::Number(f64::NAN),
                                }
                            }
                            UnaryOperator::PostDecrement => {
                                match old_val {
                                    JsValue::Number(n) => JsValue::Number(n - 1.0),
                                    _ => JsValue::Number(f64::NAN),
                                }
                            }
                            _ => unreachable!(),
                        };
                        // Update the variable/property
                        match &**operand {
                            Node::Identifier(name) => {
                                self.set_variable(name, new_val.clone())?;
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
                                self.set_property(&obj, &prop, new_val.clone())?;
                            }
                            _ => {
                                // Can't update, just return the new value
                            }
                        }
                        Ok(old_val) // Return old value for postfix
                    }
                    UnaryOperator::PreIncrement | UnaryOperator::PreDecrement => {
                        // Prefix: evaluate operand, increment/decrement, return new value
                        let val = self.evaluate_node(operand)?;
                        let new_val = match op {
                            UnaryOperator::PreIncrement => {
                                match val {
                                    JsValue::Number(n) => JsValue::Number(n + 1.0),
                                    _ => JsValue::Number(f64::NAN),
                                }
                            }
                            UnaryOperator::PreDecrement => {
                                match val {
                                    JsValue::Number(n) => JsValue::Number(n - 1.0),
                                    _ => JsValue::Number(f64::NAN),
                                }
                            }
                            _ => unreachable!(),
                        };
                        // Update the variable/property
                        match &**operand {
                            Node::Identifier(name) => {
                                self.set_variable(name, new_val.clone())?;
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
                                self.set_property(&obj, &prop, new_val.clone())?;
                            }
                            _ => {
                                // Can't update, just return the new value
                            }
                        }
                        Ok(new_val) // Return new value for prefix
                    }
                    _ => {
                        let val = self.evaluate_node(operand)?;
                        match op {
                            UnaryOperator::Not => Ok(JsValue::Boolean(!self.is_truthy(&val))),
                            UnaryOperator::Negative => {
                                match val {
                                    JsValue::Number(n) => Ok(JsValue::Number(-n)),
                                    _ => Ok(JsValue::Number(f64::NAN)),
                                }
                            }
                            UnaryOperator::Typeof => {
                                let type_str = match val {
                                    JsValue::Undefined => "undefined",
                                    JsValue::Null => "object", // JavaScript quirk
                                    JsValue::Boolean(_) => "boolean",
                                    JsValue::Number(_) => "number",
                                    JsValue::String(_) => "string",
                                    JsValue::Object(_) => "object",
                                    JsValue::Function(_) | JsValue::NativeFunction(_) => "function",
                                };
                                Ok(JsValue::String(type_str.to_string()))
                            }
                            _ => Ok(JsValue::Undefined),
                        }
                    }
                }
            }
            
            Node::Block(statements) => {
                let mut result = JsValue::Undefined;
                for stmt in statements {
                    result = self.evaluate_node(stmt)?;
                }
                Ok(result)
            }
            
            Node::ReturnStatement(expr) => {
                if let Some(e) = expr {
                    self.evaluate_node(e)
                } else {
                    Ok(JsValue::Undefined)
                }
            }
            
            Node::ForLoop { init, condition, update, body } => {
                // Initialize
                if let Some(init_node) = init {
                    self.evaluate_node(init_node)?;
                }
                
                // Loop
                let mut iterations = 0;
                const MAX_ITERATIONS: usize = 10000; // Prevent infinite loops
                
                loop {
                    // Check condition
                    if let Some(cond) = condition {
                        let cond_val = self.evaluate_node(cond)?;
                        if !self.is_truthy(&cond_val) {
                            break;
                        }
                    }
                    
                    // Execute body
                    self.evaluate_node(body)?;
                    
                    // Update
                    if let Some(upd) = update {
                        self.evaluate_node(upd)?;
                    }
                    
                    iterations += 1;
                    if iterations >= MAX_ITERATIONS {
                        log::warn!(target: "javascript", "For loop exceeded max iterations, breaking");
                        break;
                    }
                }
                Ok(JsValue::Undefined)
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
            
            Node::CallExpr { callee, arguments } => {
                debug!(target: "javascript", "Evaluating call expression");
                let callee_value = self.evaluate_node(callee)?;
                
                // Evaluate all arguments
                let mut arg_values = Vec::new();
                for arg in arguments {
                    arg_values.push(self.evaluate_node(arg)?);
                }
                
                match callee_value {
                    JsValue::NativeFunction(name) => {
                        // Handle built-in functions
                        self.call_native_function(&name, &arg_values)
                    }
                    JsValue::Function(func) => {
                        // Call user-defined function
                        self.call_function(&func, &arg_values)
                    }
                    _ => {
                        // Non-function value called - return undefined instead of error
                        debug!(target: "javascript", "Attempted to call non-function: {:?}", callee_value);
                        Ok(JsValue::Undefined)
                    }
                }
            }
            
            Node::FunctionDecl { name, params, body } => {
                // Create a function value and store it in the current scope
                let func = JsUserFunction {
                    name: Some(name.clone()),
                    params: params.clone(),
                    body: body.clone(),
                };
                self.set_variable(name, JsValue::Function(Rc::new(func)))?;
                Ok(JsValue::Undefined)
            }
            
            Node::FunctionExpr { name, params, body } => {
                // Create a function value and return it
                let func = JsUserFunction {
                    name: name.clone(),
                    params: params.clone(),
                    body: body.clone(),
                };
                Ok(JsValue::Function(Rc::new(func)))
            }
            
            Node::ArrowFunction { params, body } => {
                // Arrow functions are converted to regular functions
                let func = JsUserFunction {
                    name: None,
                    params: params.clone(),
                    body: vec![(**body).clone()],
                };
                Ok(JsValue::Function(Rc::new(func)))
            }
            
            Node::VariableDecl { name, init } => {
                let value = if let Some(init_expr) = init {
                    self.evaluate_node(init_expr)?
                } else {
                    JsValue::Undefined
                };
                self.set_variable(name, value)?;
                Ok(JsValue::Undefined)
            }
            
            Node::IfStatement { condition, consequent, alternate } => {
                let cond_value = self.evaluate_node(condition)?;
                if self.is_truthy(&cond_value) {
                    self.evaluate_node(consequent)
                } else if let Some(alt) = alternate {
                    self.evaluate_node(alt)
                } else {
                    Ok(JsValue::Undefined)
                }
            }
            
            Node::WhileLoop { condition, body } => {
                let mut iterations = 0;
                const MAX_ITERATIONS: usize = 10000;
                
                loop {
                    let cond_value = self.evaluate_node(condition)?;
                    if !self.is_truthy(&cond_value) {
                        break;
                    }
                    
                    self.evaluate_node(body)?;
                    
                    iterations += 1;
                    if iterations >= MAX_ITERATIONS {
                        log::warn!(target: "javascript", "While loop exceeded max iterations, breaking");
                        break;
                    }
                }
                Ok(JsValue::Undefined)
            }
            
            Node::ForInLoop { variable, object, body } => {
                let obj_value = self.evaluate_node(object)?;
                let mut iterations = 0;
                const MAX_ITERATIONS: usize = 10000;
                
                if let JsValue::Object(obj_ref) = obj_value {
                    let keys: Vec<String> = obj_ref.borrow().keys().cloned().collect();
                    for key in keys {
                        self.set_variable(variable, JsValue::String(key))?;
                        self.evaluate_node(body)?;
                        
                        iterations += 1;
                        if iterations >= MAX_ITERATIONS {
                            log::warn!(target: "javascript", "For-in loop exceeded max iterations, breaking");
                            break;
                        }
                    }
                }
                Ok(JsValue::Undefined)
            }
            
            Node::ForOfLoop { variable, iterable, body } => {
                let iter_value = self.evaluate_node(iterable)?;
                let mut iterations = 0;
                const MAX_ITERATIONS: usize = 10000;
                
                if let JsValue::Object(obj_ref) = iter_value {
                    // For arrays, iterate over numeric indices
                    if let Some(len) = obj_ref.borrow().get_length() {
                        for i in 0..len {
                            if let Some(elem) = obj_ref.borrow().get_element(i) {
                                self.set_variable(variable, elem.clone())?;
                                self.evaluate_node(body)?;
                            }
                            
                            iterations += 1;
                            if iterations >= MAX_ITERATIONS {
                                log::warn!(target: "javascript", "For-of loop exceeded max iterations, breaking");
                                break;
                            }
                        }
                    }
                }
                Ok(JsValue::Undefined)
            }
            
            Node::This => {
                // Return the global window object for now (simplified)
                if let Some(window) = self.get_variable("window") {
                    Ok(window)
                } else {
                    Ok(JsValue::Undefined)
                }
            }
            
            Node::BreakStatement | Node::ContinueStatement => {
                // These should be handled by loop constructs, for now just return undefined
                Ok(JsValue::Undefined)
            }
            
            Node::TryCatch { try_block, catch_param: _, catch_block, finally_block } => {
                // Execute try block
                let result = self.evaluate_node(try_block);
                
                // If error and catch block exists, execute it
                let result = match result {
                    Err(_) if catch_block.is_some() => {
                        self.evaluate_node(catch_block.as_ref().unwrap())
                    }
                    other => other,
                };
                
                // Always execute finally if present
                if let Some(finally) = finally_block {
                    self.evaluate_node(finally)?;
                }
                
                result.or(Ok(JsValue::Undefined))
            }
            
            Node::ThrowStatement(expr) => {
                let value = self.evaluate_node(expr)?;
                Err(format!("Uncaught: {:?}", value).into())
            }
            
            Node::LogicalOr { left, right } => {
                let left_val = self.evaluate_node(left)?;
                if self.is_truthy(&left_val) {
                    Ok(left_val)
                } else {
                    self.evaluate_node(right)
                }
            }
            
            Node::CommaExpr(expressions) => {
                // Evaluate all expressions, return the last one
                let mut result = JsValue::Undefined;
                for expr in expressions {
                    result = self.evaluate_node(&expr)?;
                }
                Ok(result)
            }
            
            Node::TernaryExpr { condition, true_expr, false_expr } => {
                let cond_value = self.evaluate_node(condition)?;
                if self.is_truthy(&cond_value) {
                    self.evaluate_node(true_expr)
                } else {
                    self.evaluate_node(false_expr)
                }
            }
            
            Node::Program(statements) => {
                let mut result = JsValue::Undefined;
                for stmt in statements {
                    result = self.evaluate_node(stmt)?;
                }
                Ok(result)
            }
            
            _ => Ok(JsValue::Undefined),
        }
    }
    
    fn call_native_function(&self, name: &str, args: &[JsValue]) -> Result<JsValue, Box<dyn Error>> {
        match name {
            "console.log" | "console.info" => {
                print!("[JS console.log] ");
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        print!(" ");
                    }
                    print!("{}", self.js_value_to_string(arg));
                }
                println!();
                Ok(JsValue::Undefined)
            }
            "console.warn" => {
                print!("[JS console.warn] ");
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        print!(" ");
                    }
                    print!("{}", self.js_value_to_string(arg));
                }
                println!();
                Ok(JsValue::Undefined)
            }
            "console.error" => {
                print!("[JS console.error] ");
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        print!(" ");
                    }
                    print!("{}", self.js_value_to_string(arg));
                }
                println!();
                Ok(JsValue::Undefined)
            }
            // DOM methods - actually search the DOM
            "document.getElementById" => {
                if let Some(id) = args.first().and_then(|a| match a {
                    JsValue::String(s) => Some(s.as_str()),
                    _ => None,
                }) {
                    if let Some(element) = self.find_element_by_id(id) {
                        Ok(self.create_element_object(element))
                    } else {
                        Ok(JsValue::Null)
                    }
                } else {
                    Ok(JsValue::Null)
                }
            }
            "document.querySelector" => {
                if let Some(selector) = args.first().and_then(|a| match a {
                    JsValue::String(s) => Some(s.as_str()),
                    _ => None,
                }) {
                    // Simple selector support: #id, .class, tag
                    if selector.starts_with('#') {
                        let id = &selector[1..];
                        if let Some(element) = self.find_element_by_id(id) {
                            Ok(self.create_element_object(element))
                        } else {
                            Ok(JsValue::Null)
                        }
                    } else if selector.starts_with('.') {
                        let class = &selector[1..];
                        if let Some(element) = self.find_element_by_class(class) {
                            Ok(self.create_element_object(element))
                        } else {
                            Ok(JsValue::Null)
                        }
                    } else {
                        // Tag name
                        if let Some(element) = self.find_element_by_tag(selector) {
                            Ok(self.create_element_object(element))
                        } else {
                            Ok(JsValue::Null)
                        }
                    }
                } else {
                    Ok(JsValue::Null)
                }
            }
            "document.querySelectorAll" | "document.getElementsByTagName" | "document.getElementsByClassName" => {
                // Return empty array-like object
                let mut arr = JsObject::new();
                arr.set("length", JsValue::Number(0.0));
                Ok(JsValue::Object(Rc::new(RefCell::new(arr))))
            }
            "document.createElement" => {
                // Return a stub element object
                let mut elem = JsObject::new();
                if let Some(JsValue::String(tag)) = args.first() {
                    elem.set("tagName", JsValue::String(tag.to_uppercase()));
                    elem.set("nodeName", JsValue::String(tag.to_uppercase()));
                }
                elem.set("innerHTML", JsValue::String(String::new()));
                elem.set("textContent", JsValue::String(String::new()));
                elem.set("appendChild", JsValue::NativeFunction("element.appendChild".to_string()));
                elem.set("setAttribute", JsValue::NativeFunction("element.setAttribute".to_string()));
                elem.set("getAttribute", JsValue::NativeFunction("element.getAttribute".to_string()));
                elem.set("addEventListener", JsValue::NativeFunction("element.addEventListener".to_string()));
                Ok(JsValue::Object(Rc::new(RefCell::new(elem))))
            }
            // Event handlers - no-op, just store callbacks (in a real impl we'd store them)
            "document.addEventListener" | "window.addEventListener" | "element.addEventListener" => {
                // No-op - we don't have a real event system yet
                debug!(target: "javascript", "addEventListener called (no-op)");
                Ok(JsValue::Undefined)
            }
            "document.removeEventListener" | "window.removeEventListener" | "element.removeEventListener" => {
                // No-op
                Ok(JsValue::Undefined)
            }
            "element.appendChild" | "element.removeChild" | "element.insertBefore" => {
                // Return the argument (child)
                Ok(args.first().cloned().unwrap_or(JsValue::Undefined))
            }
            "element.setAttribute" | "element.getAttribute" => {
                Ok(JsValue::Undefined)
            }
            // Window methods
            "window.setTimeout" | "window.setInterval" => {
                // Return a fake timer ID
                Ok(JsValue::Number(1.0))
            }
            "window.clearTimeout" | "window.clearInterval" => {
                Ok(JsValue::Undefined)
            }
            "window.alert" | "window.confirm" | "window.prompt" => {
                // Log and return appropriate values
                if name == "window.confirm" {
                    Ok(JsValue::Boolean(true))
                } else if name == "window.prompt" {
                    Ok(JsValue::String(String::new()))
                } else {
                    Ok(JsValue::Undefined)
                }
            }
            // JSON methods
            "JSON.parse" => {
                if let Some(JsValue::String(s)) = args.first() {
                    // Very basic JSON parsing - just return an empty object for now
                    debug!(target: "javascript", "JSON.parse called (stub)");
                    Ok(JsValue::Object(Rc::new(RefCell::new(JsObject::new()))))
                } else {
                    Ok(JsValue::Undefined)
                }
            }
            "JSON.stringify" => {
                if let Some(val) = args.first() {
                    Ok(JsValue::String(self.js_value_to_string(val)))
                } else {
                    Ok(JsValue::Undefined)
                }
            }
            // More window methods
            "window.requestAnimationFrame" | "window.cancelAnimationFrame" => {
                Ok(JsValue::Number(1.0))
            }
            "window.getComputedStyle" => {
                // Return a stub style object
                let mut style = JsObject::new();
                style.set("getPropertyValue", JsValue::NativeFunction("style.getPropertyValue".to_string()));
                Ok(JsValue::Object(Rc::new(RefCell::new(style))))
            }
            "window.matchMedia" => {
                // Return a stub MediaQueryList
                let mut mql = JsObject::new();
                mql.set("matches", JsValue::Boolean(false));
                mql.set("media", JsValue::String(args.first()
                    .and_then(|a| match a { JsValue::String(s) => Some(s.clone()), _ => None })
                    .unwrap_or_default()));
                mql.set("addEventListener", JsValue::NativeFunction("mediaQueryList.addEventListener".to_string()));
                mql.set("removeEventListener", JsValue::NativeFunction("mediaQueryList.removeEventListener".to_string()));
                Ok(JsValue::Object(Rc::new(RefCell::new(mql))))
            }
            "style.getPropertyValue" | "mediaQueryList.addEventListener" | "mediaQueryList.removeEventListener" => {
                Ok(JsValue::Undefined)
            }
            // Array methods
            "Array.isArray" => {
                if let Some(val) = args.first() {
                    match val {
                        JsValue::Object(obj) => Ok(JsValue::Boolean(obj.borrow().is_array())),
                        _ => Ok(JsValue::Boolean(false)),
                    }
                } else {
                    Ok(JsValue::Boolean(false))
                }
            }
            // Object methods
            "Object.keys" => {
                if let Some(JsValue::Object(obj)) = args.first() {
                    let keys: Vec<String> = obj.borrow().keys().cloned().collect();
                    let mut arr = JsObject::new();
                    for (i, key) in keys.iter().enumerate() {
                        arr.set(&i.to_string(), JsValue::String(key.clone()));
                    }
                    arr.set("length", JsValue::Number(keys.len() as f64));
                    Ok(JsValue::Object(Rc::new(RefCell::new(arr))))
                } else {
                    let mut arr = JsObject::new();
                    arr.set("length", JsValue::Number(0.0));
                    Ok(JsValue::Object(Rc::new(RefCell::new(arr))))
                }
            }
            _ => {
                debug!(target: "javascript", "Unknown native function: {}", name);
                Ok(JsValue::Undefined)
            }
        }
    }
    
    fn call_function(&mut self, func: &JsUserFunction, args: &[JsValue]) -> Result<JsValue, Box<dyn Error>> {
        // Create a new scope for the function
        let mut new_scope = Scope::new(None);
        
        // Bind parameters to arguments
        for (i, param) in func.params.iter().enumerate() {
            let arg_value = args.get(i).cloned().unwrap_or(JsValue::Undefined);
            new_scope.variables.insert(param.clone(), arg_value);
        }
        
        // Push the scope onto the call stack
        self.call_stack.push(new_scope);
        
        // Execute the function body
        let mut result = JsValue::Undefined;
        for stmt in &func.body {
            result = self.evaluate_node(stmt)?;
            // TODO: Handle early return statements properly
        }
        
        // Pop the scope
        self.call_stack.pop();
        
        Ok(result)
    }
    
    fn js_value_to_string(&self, value: &JsValue) -> String {
        match value {
            JsValue::Undefined => "undefined".to_string(),
            JsValue::Null => "null".to_string(),
            JsValue::Boolean(b) => b.to_string(),
            JsValue::Number(n) => n.to_string(),
            JsValue::String(s) => s.clone(),
            JsValue::Object(_) => "[object Object]".to_string(),
            JsValue::Function(_) => "[function]".to_string(),
            JsValue::NativeFunction(name) => format!("[native function {}]", name),
        }
    }

    fn evaluate_binary_op(&mut self, op: &BinaryOperator, left: &JsValue, right: &JsValue) -> Result<JsValue, Box<dyn Error>> {
        match op {
            BinaryOperator::Add => {
                match (left, right) {
                    (JsValue::Number(a), JsValue::Number(b)) => Ok(JsValue::Number(a + b)),
                    (JsValue::String(a), b) => Ok(JsValue::String(format!("{}{}", a, self.js_value_to_string(b)))),
                    (a, JsValue::String(b)) => Ok(JsValue::String(format!("{}{}", self.js_value_to_string(a), b))),
                    _ => Ok(JsValue::String(format!("{}{}", self.js_value_to_string(left), self.js_value_to_string(right)))),
                }
            }
            BinaryOperator::Subtract => {
                match (left, right) {
                    (JsValue::Number(a), JsValue::Number(b)) => Ok(JsValue::Number(a - b)),
                    _ => Ok(JsValue::Number(f64::NAN)),
                }
            }
            BinaryOperator::Multiply => {
                match (left, right) {
                    (JsValue::Number(a), JsValue::Number(b)) => Ok(JsValue::Number(a * b)),
                    _ => Ok(JsValue::Number(f64::NAN)),
                }
            }
            BinaryOperator::Divide => {
                match (left, right) {
                    (JsValue::Number(a), JsValue::Number(b)) => Ok(JsValue::Number(a / b)),
                    _ => Ok(JsValue::Number(f64::NAN)),
                }
            }
            BinaryOperator::Modulo => {
                match (left, right) {
                    (JsValue::Number(a), JsValue::Number(b)) => Ok(JsValue::Number(a % b)),
                    _ => Ok(JsValue::Number(f64::NAN)),
                }
            }
            BinaryOperator::Equal | BinaryOperator::StrictEqual => {
                Ok(JsValue::Boolean(self.js_equals(left, right)))
            }
            BinaryOperator::NotEqual | BinaryOperator::StrictNotEqual => {
                Ok(JsValue::Boolean(!self.js_equals(left, right)))
            }
            BinaryOperator::LessThan => {
                match (left, right) {
                    (JsValue::Number(a), JsValue::Number(b)) => Ok(JsValue::Boolean(a < b)),
                    (JsValue::String(a), JsValue::String(b)) => Ok(JsValue::Boolean(a < b)),
                    _ => Ok(JsValue::Boolean(false)),
                }
            }
            BinaryOperator::GreaterThan => {
                match (left, right) {
                    (JsValue::Number(a), JsValue::Number(b)) => Ok(JsValue::Boolean(a > b)),
                    (JsValue::String(a), JsValue::String(b)) => Ok(JsValue::Boolean(a > b)),
                    _ => Ok(JsValue::Boolean(false)),
                }
            }
            BinaryOperator::LessThanEqual => {
                match (left, right) {
                    (JsValue::Number(a), JsValue::Number(b)) => Ok(JsValue::Boolean(a <= b)),
                    (JsValue::String(a), JsValue::String(b)) => Ok(JsValue::Boolean(a <= b)),
                    _ => Ok(JsValue::Boolean(false)),
                }
            }
            BinaryOperator::GreaterThanEqual => {
                match (left, right) {
                    (JsValue::Number(a), JsValue::Number(b)) => Ok(JsValue::Boolean(a >= b)),
                    (JsValue::String(a), JsValue::String(b)) => Ok(JsValue::Boolean(a >= b)),
                    _ => Ok(JsValue::Boolean(false)),
                }
            }
            BinaryOperator::LogicalAnd => {
                if self.is_truthy(left) {
                    Ok(right.clone())
                } else {
                    Ok(left.clone())
                }
            }
            BinaryOperator::LogicalOr => {
                if self.is_truthy(left) {
                    Ok(left.clone())
                } else {
                    Ok(right.clone())
                }
            }
            BinaryOperator::Instanceof => {
                // Simplified instanceof - check if left is object and right is constructor
                match (left, right) {
                    (JsValue::Object(_), JsValue::Function(_)) => Ok(JsValue::Boolean(true)),
                    (JsValue::Object(_), _) => Ok(JsValue::Boolean(false)),
                    _ => Ok(JsValue::Boolean(false)),
                }
            }
            BinaryOperator::In => {
                // Check if left (property name) exists in right (object)
                match (left, right) {
                    (prop, JsValue::Object(obj)) => {
                        let prop_name = self.js_value_to_string(prop);
                        Ok(JsValue::Boolean(obj.borrow().get_property(&prop_name).is_some()))
                    }
                    _ => Ok(JsValue::Boolean(false)),
                }
            }
        }
    }
    
    fn js_equals(&self, left: &JsValue, right: &JsValue) -> bool {
        match (left, right) {
            (JsValue::Undefined, JsValue::Undefined) => true,
            (JsValue::Null, JsValue::Null) => true,
            (JsValue::Undefined, JsValue::Null) | (JsValue::Null, JsValue::Undefined) => true,
            (JsValue::Boolean(a), JsValue::Boolean(b)) => a == b,
            (JsValue::Number(a), JsValue::Number(b)) => a == b,
            (JsValue::String(a), JsValue::String(b)) => a == b,
            _ => false,
        }
    }
    
    fn is_truthy(&self, value: &JsValue) -> bool {
        match value {
            JsValue::Undefined | JsValue::Null => false,
            JsValue::Boolean(b) => *b,
            JsValue::Number(n) => *n != 0.0 && !n.is_nan(),
            JsValue::String(s) => !s.is_empty(),
            JsValue::Object(_) | JsValue::Function(_) | JsValue::NativeFunction(_) => true,
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
    
    fn get_variable(&self, name: &str) -> Option<JsValue> {
        // Look in call stack first (most recent scope first)
        for scope in self.call_stack.iter().rev() {
            if let Some(value) = scope.variables.get(name) {
                return Some(value.clone());
            }
        }
        // Then check global scope
        self.global_scope.variables.get(name).cloned()
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
                    _ => return Ok(()), // Silently ignore invalid property keys
                };
                
                // Check if this is a DOM element with innerHTML or textContent
                if prop_name == "innerHTML" || prop_name == "textContent" {
                    // Try to find the DOM node reference
                    let obj_borrow = obj_ref.borrow();
                    if let Some(JsValue::Object(dom_wrapper)) = obj_borrow.get_property("__dom_node") {
                        let wrapper_borrow = dom_wrapper.borrow();
                        if let Some(JsValue::String(_ref_str)) = wrapper_borrow.get_property("__ref") {
                            // We stored a pointer, but we can't use it directly
                            // Instead, search for the element again and update it
                            if let Some(id) = obj_borrow.get_property("id") {
                                if let JsValue::String(id_str) = id {
                                    if let Some(dom_node) = self.find_element_by_id(&id_str) {
                                        let new_value = match &value {
                                            JsValue::String(s) => s.clone(),
                                            _ => self.js_value_to_string(&value),
                                        };
                                        
                                        if prop_name == "innerHTML" {
                                            // Parse HTML and replace children
                                            // For now, just set as text content
                                            self.set_dom_text_content(&mut dom_node.borrow_mut(), &new_value);
                                        } else {
                                            // textContent - replace all text children
                                            self.set_dom_text_content(&mut dom_node.borrow_mut(), &new_value);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                
                // Always update the JS object property
                obj_ref.borrow_mut().set_property(prop_name, value);
                Ok(())
            }
            _ => {
                // In JavaScript, setting properties on primitives silently fails
                debug!(target: "javascript", "Ignoring property set on non-object: {:?}", obj);
                Ok(())
            }
        }
    }
    
    fn set_dom_text_content(&self, node: &mut DomNode, text: &str) {
        // For now, we can't directly modify the DOM structure
        // This is a limitation - we'd need to add methods to DomNode
        // or use a different approach with shared references
        // For now, just log that we're trying to set text content
        debug!(target: "javascript", "Attempting to set text content to: {}", text);
        // TODO: Implement actual DOM modification
    }

    // DOM search helper methods
    fn find_element_by_id(&self, id: &str) -> Option<Rc<RefCell<DomNode>>> {
        if let Some(root) = &self.dom_root {
            Self::search_dom_by_id(&root.borrow(), id)
        } else {
            None
        }
    }
    
    fn find_element_by_class(&self, class: &str) -> Option<Rc<RefCell<DomNode>>> {
        if let Some(root) = &self.dom_root {
            Self::search_dom_by_class(&root.borrow(), class)
        } else {
            None
        }
    }
    
    fn find_element_by_tag(&self, tag: &str) -> Option<Rc<RefCell<DomNode>>> {
        if let Some(root) = &self.dom_root {
            Self::search_dom_by_tag(&root.borrow(), tag)
        } else {
            None
        }
    }
    
    fn search_dom_by_id(node: &DomNode, id: &str) -> Option<Rc<RefCell<DomNode>>> {
        // Check if this node has the id
        if let Some(node_id) = node.get_attribute("id") {
            if node_id == id {
                return Some(Rc::new(RefCell::new(node.clone())));
            }
        }
        
        // Search children
        for child in node.children() {
            if let Some(found) = Self::search_dom_by_id(child, id) {
                return Some(found);
            }
        }
        
        None
    }
    
    fn search_dom_by_class(node: &DomNode, class: &str) -> Option<Rc<RefCell<DomNode>>> {
        // Check if this node has the class
        if let Some(class_attr) = node.get_attribute("class") {
            if class_attr.split_whitespace().any(|c| c == class) {
                return Some(Rc::new(RefCell::new(node.clone())));
            }
        }
        
        // Search children
        for child in node.children() {
            if let Some(found) = Self::search_dom_by_class(child, class) {
                return Some(found);
            }
        }
        
        None
    }
    
    fn search_dom_by_tag(node: &DomNode, tag: &str) -> Option<Rc<RefCell<DomNode>>> {
        // Check if this node matches the tag
        if node.is_element(tag) {
            return Some(Rc::new(RefCell::new(node.clone())));
        }
        
        // Search children
        for child in node.children() {
            if let Some(found) = Self::search_dom_by_tag(child, tag) {
                return Some(found);
            }
        }
        
        None
    }
    
    fn create_element_object(&self, element: Rc<RefCell<DomNode>>) -> JsValue {
        let mut elem_obj = JsObject::new();
        
        // Get element properties from DOM
        let dom_node = element.borrow();
        if let crate::dom::NodeType::Element { tag_name, .. } = dom_node.node_type() {
            elem_obj.set("tagName", JsValue::String(tag_name.to_uppercase()));
            elem_obj.set("nodeName", JsValue::String(tag_name.to_uppercase()));
            
            // Get id
            if let Some(id) = dom_node.get_attribute("id") {
                elem_obj.set("id", JsValue::String(id.to_string()));
            }
            
            // Get className
            if let Some(class) = dom_node.get_attribute("class") {
                elem_obj.set("className", JsValue::String(class.to_string()));
            }
            
            // Get innerHTML and textContent from children
            let inner_html = Self::extract_inner_html(&dom_node);
            let text_content = Self::extract_text_content(&dom_node);
            elem_obj.set("innerHTML", JsValue::String(inner_html.clone()));
            elem_obj.set("textContent", JsValue::String(text_content.clone()));
        }
        drop(dom_node);
        
        // Store reference to DOM node for property setters
        elem_obj.set("__dom_node", JsValue::Object(Rc::new(RefCell::new({
            let mut wrapper = JsObject::new();
            wrapper.set("__ref", JsValue::String(format!("{:p}", &*element.borrow())));
            wrapper
        }))));
        
        // Add methods
        elem_obj.set("setAttribute", JsValue::NativeFunction("element.setAttribute".to_string()));
        elem_obj.set("getAttribute", JsValue::NativeFunction("element.getAttribute".to_string()));
        elem_obj.set("addEventListener", JsValue::NativeFunction("element.addEventListener".to_string()));
        
        // Store the element reference in a special way so we can access it later
        // We'll use a custom property to store the Rc pointer
        JsValue::Object(Rc::new(RefCell::new(elem_obj)))
    }
    
    fn extract_inner_html(node: &DomNode) -> String {
        let mut html = String::new();
        for child in node.children() {
            match child.node_type() {
                crate::dom::NodeType::Element { tag_name, attributes, .. } => {
                    html.push('<');
                    html.push_str(tag_name);
                    for attr in attributes {
                        html.push(' ');
                        html.push_str(&attr.name);
                        html.push_str("=\"");
                        html.push_str(&attr.value);
                        html.push('"');
                    }
                    html.push('>');
                    html.push_str(&Self::extract_inner_html(&child));
                    html.push_str("</");
                    html.push_str(tag_name);
                    html.push('>');
                }
                crate::dom::NodeType::Text(text) => {
                    html.push_str(text);
                }
                crate::dom::NodeType::Comment(comment) => {
                    html.push_str("<!--");
                    html.push_str(comment);
                    html.push_str("-->");
                }
            }
        }
        html
    }
    
    fn extract_text_content(node: &DomNode) -> String {
        let mut text = String::new();
        for child in node.children() {
            match child.node_type() {
                crate::dom::NodeType::Text(t) => {
                    text.push_str(t);
                }
                crate::dom::NodeType::Element { .. } => {
                    text.push_str(&Self::extract_text_content(&child));
                }
                _ => {}
            }
        }
        text
    }

    fn get_property(&self, obj: &JsValue, prop: &JsValue) -> Result<JsValue, Box<dyn Error>> {
        let prop_name = match prop {
            JsValue::String(s) => s.to_string(),
            JsValue::Number(n) => n.to_string(),
            _ => return Ok(JsValue::Undefined),
        };
        
        match obj {
            JsValue::Object(obj_ref) => {
                Ok(obj_ref.borrow().get_property(&prop_name)
                    .cloned()
                    .unwrap_or(JsValue::Undefined))
            }
            JsValue::String(s) => {
                // String properties like .length
                match prop_name.as_str() {
                    "length" => Ok(JsValue::Number(s.len() as f64)),
                    _ => {
                        // Try to access character by index
                        if let Ok(idx) = prop_name.parse::<usize>() {
                            Ok(s.chars().nth(idx)
                                .map(|c| JsValue::String(c.to_string()))
                                .unwrap_or(JsValue::Undefined))
                        } else {
                            Ok(JsValue::Undefined)
                        }
                    }
                }
            }
            JsValue::Undefined | JsValue::Null => {
                // In JavaScript, accessing properties on null/undefined is a TypeError
                // But we'll be lenient and return undefined
                debug!(target: "javascript", "Property access on null/undefined");
                Ok(JsValue::Undefined)
            }
            _ => {
                // Other primitive types - return undefined
                debug!(target: "javascript", "Property access on primitive: {:?}", obj);
                Ok(JsValue::Undefined)
            }
        }
    }
} 