pub mod parser;
pub mod tokenizer;
pub mod entities;

pub struct HtmlDocument {
    pub doctype: Option<String>,
    pub root_element: Option<crate::dom::Node>,
    pub head: Option<crate::dom::Node>,
    pub body: Option<crate::dom::Node>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HtmlVersion {
    Html5,
    Html4Strict,
    Html4Transitional,
    XHtml1Strict,
    XHtml1Transitional,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Namespace {
    Html,
    Svg,
    MathMl,
    Xml,
    XLink,
    Custom(String),
}

impl HtmlDocument {
    pub fn new() -> Self {
        Self {
            doctype: None,
            root_element: None,
            head: None,
            body: None,
        }
    }

    pub fn set_doctype(&mut self, doctype: String) {
        self.doctype = Some(doctype);
    }

    pub fn set_root_element(&mut self, element: crate::dom::Node) {
        self.root_element = Some(element);
    }

    pub fn set_head(&mut self, head: crate::dom::Node) {
        self.head = Some(head);
    }

    pub fn set_body(&mut self, body: crate::dom::Node) {
        self.body = Some(body);
    }
}
