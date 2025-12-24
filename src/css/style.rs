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

        for rule in self.stylesheet.style_rules() {
            if let super::Rule::StyleRule { selectors, declarations } = rule {
                if selectors.iter().any(|selector| self.matches_selector(node, selector)) {
                    styled_node.add_declarations(declarations.clone());
                }
            }
        }

        styled_node
    }

    fn matches_selector(&self, node: &Node, selector: &Selector) -> bool {
        match node.node_type() {
            NodeType::Element {
                tag_name,
                attributes,
                ..
            } => self.matches_complex_selector(node, selector, attributes),
            _ => false,
        }
    }

    fn matches_complex_selector(
        &self,
        node: &Node,
        selector: &Selector,
        attributes: &[crate::dom::Attribute],
    ) -> bool {
        // For now, check if any component matches - this is a simplified implementation
        // A full implementation would need to handle combinator logic
        selector.components.iter().any(|component| {
            match component {
                SelectorComponent::Type(name) => {
                    if let NodeType::Element { tag_name, .. } = node.node_type() {
                        name.eq_ignore_ascii_case(name)
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
                SelectorComponent::PseudoClass(pseudo) => {
                    // Basic pseudo-class support - simplified for now since we don't have parent access
                    match pseudo.as_str() {
                        "hover" | "active" | "focus" | "visited" => false, // These need interaction state
                        "first-child" | "last-child" => false, // Would need parent access
                        _ => false,
                    }
                }
                SelectorComponent::PseudoElement(_) => false, // Pseudo-elements need special handling
                SelectorComponent::Attribute(attr_name, attr_value) => {
                    if let Some(expected_value) = attr_value {
                        attributes.iter().any(|attr| {
                            attr.name == *attr_name && attr.value == *expected_value
                        })
                    } else {
                        attributes.iter().any(|attr| attr.name == *attr_name)
                    }
                }
                _ => false,
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
