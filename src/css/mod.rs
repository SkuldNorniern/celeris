pub mod parser;
pub mod selector;
pub mod style;
pub mod values;
pub mod rules;
pub mod properties;
pub mod test_cases;

// Re-export main types for convenience
pub use rules::{StyleSheet, Rule, Declaration, AtRule, Keyframe};
pub use values::{Value, Color, Unit};
pub use properties::Property;
pub use selector::{Selector, SelectorComponent, Specificity};

// Legacy re-exports for backward compatibility
pub use rules::StyleSheet as CssStyleSheet;
pub use rules::Rule as CssRule;
pub use rules::Declaration as CssDeclaration;
