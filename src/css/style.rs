use super::selector::{Selector, SelectorComponent};
use super::{Declaration, Rule, StyleSheet};
use crate::dom::{Node, NodeType};

pub struct StyleEngine {
    stylesheet: StyleSheet,
}

impl StyleEngine {
    pub fn new(stylesheet: StyleSheet) -> Self {
        Self { stylesheet }
    }

    pub fn apply_styles(&self, node: &Node) -> StyledNode {
        let mut styled_node = StyledNode::new(node.clone());

        for rule in self.stylesheet.rules() {
            if self.matches_selector(node, rule) {
                styled_node.add_declarations(rule.declarations.clone());
            }
        }

        styled_node
    }

    fn matches_selector(&self, node: &Node, rule: &Rule) -> bool {
        match node.node_type() {
            NodeType::Element {
                tag_name,
                attributes,
                ..
            } => rule
                .selectors
                .iter()
                .any(|selector| self.matches_complex_selector(node, selector, attributes)),
            _ => false,
        }
    }

    fn matches_complex_selector(
        &self,
        node: &Node,
        selector: &Selector,
        attributes: &[crate::dom::Attribute],
    ) -> bool {
        selector.components.iter().any(|component| {
            match component {
                SelectorComponent::Type(name) => {
                    if let NodeType::Element { tag_name, .. } = node.node_type() {
                        name == tag_name
                    } else {
                        false
                    }
                }
                SelectorComponent::Id(id) => attributes
                    .iter()
                    .any(|attr| attr.name == "id" && attr.value == *id),
                SelectorComponent::Class(class_name) => attributes.iter().any(|attr| {
                    attr.name == "class" && attr.value.split_whitespace().any(|c| c == class_name)
                }),
                SelectorComponent::Universal => true,
                _ => false, // TODO: Implement other selector types
            }
        })
    }
}

#[derive(Clone)]
pub struct StyledNode {
    pub node: Node,
    pub styles: Vec<Declaration>,
}

impl StyledNode {
    pub fn new(node: Node) -> Self {
        Self {
            node,
            styles: Vec::new(),
        }
    }

    pub fn add_declarations(&mut self, declarations: Vec<Declaration>) {
        self.styles.extend(declarations);
    }

    pub fn get_style(&self, property: &str) -> Option<&Declaration> {
        self.styles.iter().find(|decl| decl.property == property)
    }
}
