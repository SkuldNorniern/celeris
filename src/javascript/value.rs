#[derive(Debug, Clone)]
pub enum JsValue {
    Undefined,
    Null,
    Boolean(bool),
    Number(f64),
    String(String),
    Object(JsObject),
    Function(JsFunction),
}

#[derive(Debug, Clone)]
pub struct JsObject {
    properties: HashMap<String, JsValue>,
    prototype: Option<Box<JsObject>>,
}

#[derive(Debug, Clone)]
pub struct JsFunction {
    name: Option<String>,
    params: Vec<String>,
    body: Vec<Node>,
    closure: Scope,
} 