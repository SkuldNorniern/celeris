use super::tokenizer::{Token, Tokenizer};
use crate::dom::{DomTree, Node, NodeType};
use log::{debug, info};

pub struct Parser {
    tokenizer: Tokenizer,
}

impl Parser {
    pub fn new(html: String) -> Self {
        debug!(target: "html", "Creating new HTML parser");
        Self {
            tokenizer: Tokenizer::new(html),
        }
    }

    pub fn parse(&mut self) -> DomTree {
        info!(target: "html", "Starting HTML parsing");
        let mut dom = DomTree::new();
        // Build a simple DOM tree with a stack of open elements.
        //
        // We keep a document root to avoid duplicating <html> when the input contains
        // an explicit <html> element (most pages do).
        let mut stack: Vec<Node> = Vec::new();
        stack.push(Node::new(NodeType::Element {
            tag_name: String::from("#document"),
            attributes: Vec::new(),
            events: Vec::new(),
        }));

        while let Some(token) = self.tokenizer.next_token() {
            debug!(target: "html", "Processing token: {:?}", token);
            match token {
                Token::StartTag { name, attributes } => {
                    debug!(target: "html", "Found start tag: <{}>", name);
                    let new_node = Node::new(NodeType::Element {
                        tag_name: name.clone(),
                        attributes,
                        events: Vec::new(),
                    });

                    if !is_void_element(&name) {
                        stack.push(new_node);
                    } else {
                        if let Some(parent) = stack.last_mut() {
                            parent.add_child(new_node);
                        }
                    }
                }
                Token::EndTag { name } => {
                    debug!(
                        target: "html",
                        "Found end tag: </{}> (open elements: {})",
                        name,
                        stack.len()
                    );

                    if is_void_element(&name) {
                        continue;
                    }

                    // Pop and attach nodes until we close a matching start tag, or we hit the
                    // document root (basic error recovery for mismatched tags).
                    while stack.len() > 1 {
                        let Some(node) = stack.pop() else {
                            break;
                        };
                        let is_match = node_is_element_named(&node, &name);
                        if let Some(parent) = stack.last_mut() {
                            parent.add_child(node);
                        }
                        if is_match {
                            break;
                        }
                    }
                }
                Token::Text(content) => {
                    if !content.trim().is_empty() {
                        debug!(target: "html", "Found text node: {}", 
                            content.chars().take(30).collect::<String>());
                        let text_node = Node::new(NodeType::Text(content));
                        if let Some(parent) = stack.last_mut() {
                            parent.add_child(text_node);
                        }
                    }
                }
                Token::Comment(content) => {
                    debug!(target: "html", "Found comment: {}", 
                        content.chars().take(30).collect::<String>());
                    let comment_node = Node::new(NodeType::Comment(content));
                    if let Some(parent) = stack.last_mut() {
                        parent.add_child(comment_node);
                    }
                }
                Token::Doctype(_) => {
                    // Just ignore doctype for now
                }
            }
        }

        // Close any still-open elements.
        while stack.len() > 1 {
            let Some(node) = stack.pop() else {
                break;
            };
            if let Some(parent) = stack.last_mut() {
                parent.add_child(node);
            }
        }

        if let Some(root) = stack.pop() {
            dom.set_root(root);
        }
        info!(target: "html", "HTML parsing complete");
        dom
    }
}

fn is_void_element(tag_name: &str) -> bool {
    matches!(
        tag_name.to_lowercase().as_str(),
        "area"
            | "base"
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "param"
            | "source"
            | "track"
            | "wbr"
    )
}

fn node_is_element_named(node: &Node, expected: &str) -> bool {
    match node.node_type() {
        NodeType::Element { tag_name, .. } => tag_name.eq_ignore_ascii_case(expected),
        _ => false,
    }
}
