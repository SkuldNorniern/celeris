pub mod parser;
pub mod selector;
pub mod style;

#[derive(Default)]
pub struct StyleSheet {
    rules: Vec<Rule>,
}

#[derive(Clone)]
pub struct Rule {
    pub selectors: Vec<selector::Selector>,
    pub declarations: Vec<Declaration>,
}

#[derive(Clone)]
pub struct Declaration {
    pub property: String,
    pub value: Value,
}

#[derive(Clone)]
pub enum Value {
    Keyword(String),
    Length(f32, Unit),
    Color(Color),
    None,
}

#[derive(Clone)]
pub enum Unit {
    Px,
    Em,
    Rem,
    Percent,
}

#[derive(Clone)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
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
}
