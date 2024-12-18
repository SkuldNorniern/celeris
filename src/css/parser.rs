use super::selector::{Selector, SelectorComponent};
use super::{Color, Declaration, Rule, StyleSheet, Unit, Value};

pub struct CssParser {
    input: String,
    position: usize,
}

impl CssParser {
    pub fn new(input: String) -> Self {
        Self { input, position: 0 }
    }

    pub fn parse(&mut self) -> StyleSheet {
        let mut stylesheet = StyleSheet::new();
        self.consume_whitespace();

        while self.position < self.input.len() {
            if let Some(rule) = self.parse_rule() {
                stylesheet.add_rule(rule);
            }
            self.consume_whitespace();
        }

        stylesheet
    }

    fn parse_rule(&mut self) -> Option<Rule> {
        let selectors = self.parse_selectors()?;
        self.consume_whitespace();

        if !self.expect_char('{') {
            return None;
        }

        let declarations = self.parse_declarations();

        if !self.expect_char('}') {
            return None;
        }

        Some(Rule {
            selectors,
            declarations,
        })
    }

    fn parse_selectors(&mut self) -> Option<Vec<Selector>> {
        let mut selectors = Vec::new();

        loop {
            self.consume_whitespace();
            let components = self.parse_selector_components()?;
            selectors.push(Selector::new(components));

            self.consume_whitespace();
            if !self.expect_char(',') {
                break;
            }
        }

        Some(selectors)
    }

    fn parse_selector_components(&mut self) -> Option<Vec<SelectorComponent>> {
        let mut components = Vec::new();

        loop {
            self.consume_whitespace();
            if self.eof() || self.peek_char() == '{' || self.peek_char() == ',' {
                break;
            }

            match self.peek_char() {
                '#' => {
                    self.next_char();
                    components.push(SelectorComponent::Id(self.parse_identifier()));
                }
                '.' => {
                    self.next_char();
                    components.push(SelectorComponent::Class(self.parse_identifier()));
                }
                '*' => {
                    self.next_char();
                    components.push(SelectorComponent::Universal);
                }
                '>' => {
                    self.next_char();
                    components.push(SelectorComponent::Child);
                }
                '+' => {
                    self.next_char();
                    components.push(SelectorComponent::Adjacent);
                }
                c if c.is_ascii_alphabetic() => {
                    components.push(SelectorComponent::Type(self.parse_identifier()));
                }
                _ => break,
            }
        }

        if components.is_empty() {
            None
        } else {
            Some(components)
        }
    }

    fn parse_declarations(&mut self) -> Vec<Declaration> {
        let mut declarations = Vec::new();

        loop {
            self.consume_whitespace();
            if self.peek_char() == '}' {
                break;
            }

            if let Some(declaration) = self.parse_declaration() {
                declarations.push(declaration);
            }

            self.consume_whitespace();
            self.expect_char(';');
        }

        declarations
    }

    fn parse_declaration(&mut self) -> Option<Declaration> {
        let property = self.parse_identifier();
        self.consume_whitespace();

        if !self.expect_char(':') {
            return None;
        }

        self.consume_whitespace();
        let value = self.parse_value()?;

        Some(Declaration { property, value })
    }

    fn parse_value(&mut self) -> Option<Value> {
        match self.peek_char() {
            '0'..='9' => self.parse_length(),
            '#' => self.parse_color(),
            _ => Some(Value::Keyword(self.parse_identifier())),
        }
    }

    fn parse_length(&mut self) -> Option<Value> {
        let num = self
            .consume_while(|c| c.is_ascii_digit() || c == '.')
            .parse()
            .ok()?;
        let unit = match &self.parse_identifier().to_lowercase()[..] {
            "px" => Unit::Px,
            "em" => Unit::Em,
            "rem" => Unit::Rem,
            "%" => Unit::Percent,
            _ => return None,
        };
        Some(Value::Length(num, unit))
    }

    fn parse_color(&mut self) -> Option<Value> {
        self.next_char(); // consume '#'
        let hex = self.consume_while(|c| c.is_ascii_hexdigit());
        if hex.len() != 6 {
            return None;
        }

        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;

        Some(Value::Color(Color { r, g, b, a: 255 }))
    }

    fn parse_identifier(&mut self) -> String {
        self.consume_while(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    }

    fn consume_whitespace(&mut self) {
        self.consume_while(|c| c.is_whitespace());
    }

    fn consume_while<F>(&mut self, test: F) -> String
    where
        F: Fn(char) -> bool,
    {
        let mut result = String::new();
        while !self.eof() && test(self.peek_char()) {
            result.push(self.next_char());
        }
        result
    }

    fn next_char(&mut self) -> char {
        let mut iter = self.input[self.position..].char_indices();
        let (_, cur_char) = iter.next().unwrap();
        let (next_pos, _) = iter.next().unwrap_or((1, ' '));
        self.position += next_pos;
        cur_char
    }

    fn peek_char(&self) -> char {
        self.input[self.position..].chars().next().unwrap_or('\0')
    }

    fn expect_char(&mut self, expected: char) -> bool {
        if self.peek_char() == expected {
            self.next_char();
            true
        } else {
            false
        }
    }

    fn eof(&self) -> bool {
        self.position >= self.input.len()
    }
}
