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
        let mut root = Node::new(NodeType::Element {
            tag_name: String::from("html"),
            attributes: Vec::new(),
        });
        let current_node = &mut root;
        let mut ancestor_stack = Vec::new();

        while let Some(token) = self.tokenizer.next_token() {
            debug!(target: "html", "Processing token: {:?}", token);
            match token {
                Token::StartTag { name, attributes } => {
                    debug!(target: "html", "Found start tag: <{}>", name);
                    let new_node = Node::new(NodeType::Element {
                        tag_name: name.clone(),
                        attributes,
                    });

                    if !is_void_element(&name) {
                        let temp = std::mem::replace(current_node, new_node);
                        ancestor_stack.push(temp);
                    } else {
                        current_node.add_child(new_node);
                    }
                }
                Token::EndTag { name } => {
                    debug!(target: "html", "Found end tag: </{}> (stack size: {})", 
                        name, ancestor_stack.len());
                    if !is_void_element(&name) {
                        if let Some(parent) = ancestor_stack.pop() {
                            let completed_node = std::mem::replace(current_node, parent);
                            current_node.add_child(completed_node);
                        }
                    }
                }
                Token::Text(content) => {
                    if !content.trim().is_empty() {
                        debug!(target: "html", "Found text node: {}", 
                            content.chars().take(30).collect::<String>());
                        let text_node = Node::new(NodeType::Text(content));
                        current_node.add_child(text_node);
                    }
                }
                Token::Comment(content) => {
                    debug!(target: "html", "Found comment: {}", 
                        content.chars().take(30).collect::<String>());
                    let comment_node = Node::new(NodeType::Comment(content));
                    current_node.add_child(comment_node);
                }
                Token::Doctype(_) => {
                    // Just ignore doctype for now
                }
            }
        }

        dom.set_root(root);
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
