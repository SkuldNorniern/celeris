use super::tokenizer::{Token, Tokenizer};
use crate::dom::{DomTree, Node, NodeType};

pub struct Parser {
    tokenizer: Tokenizer,
    current_node: Option<Node>,
}

impl Parser {
    pub fn new(html: String) -> Self {
        Self {
            tokenizer: Tokenizer::new(html),
            current_node: None,
        }
    }

    pub fn parse(&mut self) -> DomTree {
        let mut dom = DomTree::new();
        let mut node_stack: Vec<Node> = Vec::new();

        while let Some(token) = self.tokenizer.next_token() {
            match token {
                Token::StartTag { name, attributes } => {
                    let node = Node::new(NodeType::Element {
                        tag_name: name,
                        attributes,
                    });

                    if let Some(parent) = node_stack.last_mut() {
                        let node_clone = node.clone();
                        parent.add_child(node_clone);
                    } else {
                        dom.set_root(node.clone());
                    }

                    node_stack.push(node);
                }
                Token::EndTag { name: _ } => {
                    node_stack.pop();
                }
                Token::Text(content) => {
                    if let Some(parent) = node_stack.last_mut() {
                        parent.add_child(Node::new(NodeType::Text(content)));
                    }
                }
                _ => {} // Handle other token types
            }
        }

        dom
    }
}
