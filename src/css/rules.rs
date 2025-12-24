//! CSS rule types and structures

use super::selector::Selector;
use super::values::Value;

/// CSS rule types
#[derive(Clone, Debug, PartialEq)]
pub enum Rule {
    StyleRule {
        selectors: Vec<Selector>,
        declarations: Vec<Declaration>,
    },
    AtRule(AtRule),
}

/// At-rule types
#[derive(Clone, Debug, PartialEq)]
pub enum AtRule {
    Media {
        condition: String,
        rules: Vec<Box<Rule>>,
    },
    Keyframes {
        name: String,
        keyframes: Vec<Keyframe>,
    },
    FontFace {
        declarations: Vec<Declaration>,
    },
    Import {
        url: String,
    },
    Supports {
        condition: String,
        rules: Vec<Box<Rule>>,
    },
}

/// CSS declaration (property-value pair)
#[derive(Clone, Debug, PartialEq)]
pub struct Declaration {
    pub property: String,
    pub value: Value,
    pub important: bool, // For !important declarations
}

/// Keyframe rule for animations
#[derive(Clone, Debug, PartialEq)]
pub struct Keyframe {
    pub selector: String, // "0%", "50%", "100%", "from", "to"
    pub declarations: Vec<Declaration>,
}

/// CSS stylesheet containing all rules
#[derive(Clone, Debug, Default)]
pub struct StyleSheet {
    rules: Vec<Rule>,
}

impl StyleSheet {
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    pub fn add_rule(&mut self, rule: Rule) {
        self.rules.push(rule);
    }

    pub fn rules(&self) -> &[Rule] {
        &self.rules
    }

    /// Get all style rules (filtering out at-rules)
    pub fn style_rules(&self) -> Vec<&Rule> {
        self.rules.iter().filter(|rule| matches!(rule, Rule::StyleRule { .. })).collect()
    }

    /// Get all at-rules
    pub fn at_rules(&self) -> Vec<&Rule> {
        self.rules.iter().filter(|rule| matches!(rule, Rule::AtRule(_))).collect()
    }

    /// Get media rules specifically
    pub fn media_rules(&self) -> Vec<&AtRule> {
        self.at_rules().iter().filter_map(|rule| {
            if let Rule::AtRule(at_rule) = rule {
                if matches!(at_rule, AtRule::Media { .. }) {
                    Some(at_rule)
                } else {
                    None
                }
            } else {
                None
            }
        }).collect()
    }

    /// Get keyframes rules
    pub fn keyframes_rules(&self) -> Vec<&AtRule> {
        self.at_rules().iter().filter_map(|rule| {
            if let Rule::AtRule(at_rule) = rule {
                if matches!(at_rule, AtRule::Keyframes { .. }) {
                    Some(at_rule)
                } else {
                    None
                }
            } else {
                None
            }
        }).collect()
    }
}

impl Declaration {
    pub fn new(property: String, value: Value) -> Self {
        Self {
            property,
            value,
            important: false,
        }
    }

    pub fn important(mut self, important: bool) -> Self {
        self.important = important;
        self
    }
}

impl Default for Declaration {
    fn default() -> Self {
        Self {
            property: String::new(),
            value: Value::None,
            important: false,
        }
    }
}

