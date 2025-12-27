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
        log::trace!(target: "css", "Starting CSS parsing, input length: {}", self.input.len());
        let mut stylesheet = StyleSheet::new();
        self.consume_whitespace();
        let mut rule_count = 0;
        const MAX_RULES: usize = 1000; // Prevent parsing too many rules
        let mut consecutive_failures = 0;
        const MAX_CONSECUTIVE_FAILURES: usize = 10; // Stop after too many failed rule parses

        while self.position < self.input.len() && rule_count < MAX_RULES && consecutive_failures < MAX_CONSECUTIVE_FAILURES {
            log::trace!(target: "css", "Parse loop: position={}, remaining={}", self.position, self.input.len() - self.position);
            if let Some(rule) = self.parse_rule() {
                stylesheet.add_rule(rule);
                rule_count += 1;
                consecutive_failures = 0; // Reset failure count on success
                log::trace!(target: "css", "Parsed rule {}, total rules: {}", rule_count, stylesheet.rules().len());
            } else {
                consecutive_failures += 1;
                log::trace!(target: "css", "parse_rule returned None (failure #{})", consecutive_failures);

                // Try to skip to next potential rule by consuming until we find whitespace or end
                if consecutive_failures >= MAX_CONSECUTIVE_FAILURES {
                    log::warn!(target: "css", "Too many consecutive parse failures ({}), stopping", consecutive_failures);
                    break;
                }

                // Skip problematic character and continue
                if !self.eof() {
                    self.next_char();
                }
            }
            self.consume_whitespace();
        }

        if rule_count >= MAX_RULES {
            log::warn!(target: "css", "Reached maximum rule limit ({}), stopping parsing", MAX_RULES);
        }

        log::trace!(target: "css", "CSS parsing complete, {} rules parsed", stylesheet.rules().len());
        stylesheet
    }

    fn parse_rule(&mut self) -> Option<Rule> {
        let start_pos = self.position;
        log::trace!(target: "css", "parse_rule: position={}, peek='{}'", self.position, self.peek_char());

        // Check for at-rule
        if self.peek_char() == '@' {
            return self.parse_at_rule();
        }

        // Regular style rule
        let selectors = match self.parse_selectors() {
            Some(s) if !s.is_empty() => s,
            _ => {
                log::trace!(target: "css", "parse_rule: no selectors found, returning None");
                return None;
            }
        };
        self.consume_whitespace();

        if !self.expect_char('{') {
            log::trace!(target: "css", "parse_rule: expected '{{', found '{}'", self.peek_char());
            return None;
        }

        let declarations = self.parse_declarations();

        if !self.expect_char('}') {
            log::trace!(target: "css", "parse_rule: expected '}}', found '{}'", self.peek_char());
            return None;
        }

        log::trace!(target: "css", "parse_rule: successfully parsed rule with {} selectors and {} declarations",
                   selectors.len(), declarations.len());

        // Safety check: ensure we made progress
        // If position didn't advance, we might be stuck - return None to prevent infinite loops
        if self.position == start_pos && !self.eof() {
            log::warn!(target: "css", "parse_rule: no progress made, position stuck at {}", self.position);
            return None;
        }

        Some(Rule::StyleRule {
            selectors,
            declarations,
        })
    }

    fn parse_at_rule(&mut self) -> Option<Rule> {
        self.next_char(); // consume '@'
        let rule_type = self.parse_identifier();

        match rule_type.to_lowercase().as_str() {
            "media" => self.parse_media_rule(),
            "keyframes" => self.parse_keyframes_rule(),
            "font-face" => self.parse_font_face_rule(),
            "import" => self.parse_import_rule(),
            _ => {
                // Skip unknown at-rules
                self.skip_to_closing_brace();
                None
            }
        }
    }

    fn parse_media_rule(&mut self) -> Option<Rule> {
        self.consume_whitespace();
        let condition = match self.parse_until_char('{') {
            Some(cond) => cond,
            None => return None, // Failed to find opening brace or exceeded limits
        };
        if !self.expect_char('{') {
            return None;
        }

        let mut rules = Vec::new();
        let mut rule_iterations = 0;
        const MAX_MEDIA_RULES: usize = 100; // Prevent excessive nested rules

        while self.peek_char() != '}' && !self.eof() && rule_iterations < MAX_MEDIA_RULES {
            if let Some(rule) = self.parse_rule() {
                rules.push(Box::new(rule));
                rule_iterations += 1;
            } else {
                break; // Stop if we can't parse more rules
            }
        }

        if rule_iterations >= MAX_MEDIA_RULES {
            log::warn!(target: "css", "Media rule exceeded maximum nested rules ({}), stopping", MAX_MEDIA_RULES);
        }

        if !self.expect_char('}') {
            return None;
        }
        Some(Rule::AtRule(super::AtRule::Media { condition, rules }))
    }

    fn parse_keyframes_rule(&mut self) -> Option<Rule> {
        self.consume_whitespace();
        let name = self.parse_identifier();
        self.consume_whitespace();
        if !self.expect_char('{') {
            return None;
        }

        let mut keyframes = Vec::new();
        let mut keyframe_iterations = 0;
        const MAX_KEYFRAMES: usize = 100; // Prevent excessive keyframes

        while self.peek_char() != '}' && !self.eof() && keyframe_iterations < MAX_KEYFRAMES {
            let selector = self.parse_identifier();
            self.consume_whitespace();
            if !self.expect_char('{') {
                return None;
            }

            let declarations = self.parse_declarations();
            if !self.expect_char('}') {
                return None;
            }

            keyframes.push(super::Keyframe { selector, declarations });
            keyframe_iterations += 1;
        }

        if keyframe_iterations >= MAX_KEYFRAMES {
            log::warn!(target: "css", "Keyframes rule exceeded maximum keyframes ({}), stopping", MAX_KEYFRAMES);
        }

        if !self.expect_char('}') {
            return None;
        }
        Some(Rule::AtRule(super::AtRule::Keyframes { name, keyframes }))
    }

    fn parse_font_face_rule(&mut self) -> Option<Rule> {
        self.consume_whitespace();
        if !self.expect_char('{') {
            return None;
        }
        let declarations = self.parse_declarations();
        if !self.expect_char('}') {
            return None;
        }
        Some(Rule::AtRule(super::AtRule::FontFace { declarations }))
    }

    fn parse_import_rule(&mut self) -> Option<Rule> {
        self.consume_whitespace();
        let url = if self.peek_char() == '"' || self.peek_char() == '\'' {
            self.next_char();
            let url = self.consume_while(|c| c != '"' && c != '\'');
            self.next_char(); // consume closing quote
            url
        } else {
            self.parse_identifier()
        };
        Some(Rule::AtRule(super::AtRule::Import { url }))
    }

    fn parse_until_char(&mut self, target: char) -> Option<String> {
        let mut result = String::new();
        let start_pos = self.position;
        let mut iterations = 0;
        const MAX_LENGTH: usize = 10000; // Prevent excessive memory usage

        while self.peek_char() != target && !self.eof() && result.len() < MAX_LENGTH && iterations < MAX_LENGTH {
            result.push(self.next_char());
            iterations += 1;

            // Safety check: if we haven't moved, break to prevent infinite loop
            if self.position == start_pos && iterations > 0 {
                log::warn!(target: "css", "parse_until_char: position not advancing, breaking to prevent infinite loop");
                return None;
            }
        }

        if result.len() >= MAX_LENGTH {
            log::warn!(target: "css", "parse_until_char: exceeded maximum length ({}), stopping", MAX_LENGTH);
            return None;
        }

        if self.peek_char() != target {
            // Didn't find the target character
            return None;
        }

        Some(result.trim().to_string())
    }

    fn skip_to_closing_brace(&mut self) {
        let mut brace_count = 0;
        while !self.eof() {
            match self.peek_char() {
                '{' => {
                    brace_count += 1;
                    self.next_char();
                }
                '}' => {
                    brace_count -= 1;
                    self.next_char();
                    if brace_count <= 0 {
                        break;
                    }
                }
                _ => {
                    self.next_char();
                }
            }
        }
    }

    fn parse_selectors(&mut self) -> Option<Vec<Selector>> {
        let mut selectors = Vec::new();
        let mut iterations = 0;
        const MAX_ITERATIONS: usize = 50; // Prevent too many selectors

        loop {
            iterations += 1;
            if iterations > MAX_ITERATIONS {
                log::warn!(target: "css", "Selector parsing exceeded maximum iterations, stopping");
                break;
            }

            self.consume_whitespace();
            let components = match self.parse_selector_components() {
                Some(c) if !c.is_empty() => c,
                _ => break,
            };
            selectors.push(Selector::new(components));

            self.consume_whitespace();
            if !self.expect_char(',') {
                break;
            }
        }

        if selectors.is_empty() {
            None
        } else {
            Some(selectors)
        }
    }

    fn parse_selector_components(&mut self) -> Option<Vec<SelectorComponent>> {
        let mut components = Vec::new();
        let mut iterations = 0;
        const MAX_ITERATIONS: usize = 100; // Prevent infinite loops

        loop {
            iterations += 1;
            if iterations > MAX_ITERATIONS {
                log::warn!(target: "css", "Selector parsing exceeded maximum iterations, stopping to prevent infinite loop");
                break;
            }

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
                ':' => {
                    self.next_char();
                    if self.peek_char() == ':' {
                        // Pseudo-element
                        self.next_char();
                        let pseudo_name = self.parse_identifier();
                        if !pseudo_name.is_empty() {
                            components.push(SelectorComponent::PseudoElement(pseudo_name));
                        }
                    } else {
                        // Pseudo-class
                        let pseudo_name = self.parse_identifier();
                        if !pseudo_name.is_empty() {
                            components.push(SelectorComponent::PseudoClass(pseudo_name));
                        }
                    }
                }
                '[' => {
                    self.next_char();
                    let attr_name = self.parse_identifier();
                    if attr_name.is_empty() {
                        break;
                    }
                    self.consume_whitespace();

                    let attr_value = if self.peek_char() == '=' || self.peek_char() == '~' || self.peek_char() == '|' || self.peek_char() == '^' || self.peek_char() == '$' || self.peek_char() == '*' {
                        let operator = self.next_char();
                        self.consume_whitespace();
                        if self.peek_char() == '"' || self.peek_char() == '\'' {
                            let quote_char = self.next_char();
                            let value = self.consume_while(|c| c != quote_char);
                            if self.peek_char() == quote_char {
                                self.next_char(); // consume closing quote
                                Some(format!("{}{}", operator, value))
                            } else {
                                // Malformed - no closing quote, use what we have
                                Some(format!("{}{}", operator, value))
                            }
                        } else {
                            let value = self.parse_identifier();
                            Some(format!("{}{}", operator, value))
                        }
                    } else {
                        None
                    };

                    if self.expect_char(']') {
                        components.push(SelectorComponent::Attribute(attr_name, attr_value));
                    }
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
        let mut iterations = 0;
        // Increased limit to handle large CSS files (e.g., Google's CSS)
        // With proper parsing logic, this should not cause memory issues
        const MAX_DECLARATIONS: usize = 10000; // Increased from 1000 to handle real-world CSS

        loop {
            iterations += 1;
            if iterations > MAX_DECLARATIONS {
                log::warn!(target: "css", "Exceeded maximum declarations ({}), stopping", MAX_DECLARATIONS);
                break;
            }

            self.consume_whitespace();
            if self.peek_char() == '}' {
                break;
            }

            let pos_before = self.position;
            if let Some(declaration) = self.parse_declaration() {
                declarations.push(declaration);
            } else {
                // If parsing failed, ensure we make progress to avoid infinite loops
                if self.position == pos_before && !self.eof() {
                    // Skip the problematic character
                    self.next_char();
                }
            }

            self.consume_whitespace();
            
            // Check for closing brace before expecting semicolon
            if self.peek_char() == '}' {
                break;
            }
            
            // Try to consume semicolon, but don't fail if it's missing
            if self.peek_char() == ';' {
                self.next_char();
            }
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

        // Check for !important
        let important = self.check_important();

        Some(Declaration { property, value, important })
    }

    fn check_important(&mut self) -> bool {
        self.consume_whitespace();
        if self.peek_char() == '!' {
            self.next_char();
            self.consume_whitespace();
            let important_keyword = self.parse_identifier();
            important_keyword.to_lowercase() == "important"
        } else {
            false
        }
    }

    fn parse_value(&mut self) -> Option<Value> {
        // Skip whitespace
        self.consume_whitespace();
        
        if self.eof() {
            return None;
        }

        // Try to parse first value
        let first_value = match self.peek_char() {
            '0'..='9' => self.parse_length(),
            '#' => self.parse_color(),
            '(' => self.parse_function(),
            'v' => self.parse_var_reference(),
            _ => self.parse_single_value(),
        }?;
        
        self.consume_whitespace();

        // Check for multiple values (space-separated or comma-separated)
        if !self.eof() && self.peek_char() != '\0' && self.peek_char() != ';' && self.peek_char() != '}' && self.peek_char() != ')' {
            // Multiple values
            let mut values = vec![first_value.clone()];
            let mut iterations = 0;
            const MAX_VALUES: usize = 100;
            
            while !self.eof() && self.peek_char() != '\0' && self.peek_char() != ';' && self.peek_char() != '}' && self.peek_char() != ')' && iterations < MAX_VALUES {
                iterations += 1;
                if let Some(val) = self.parse_single_value() {
                    values.push(val);
                } else {
                    break;
                }
                self.consume_whitespace();
            }
            
            if values.len() > 1 {
                Some(Value::Multiple(values))
            } else {
                Some(first_value)
            }
        } else {
            Some(first_value)
        }
    }

    fn parse_single_value(&mut self) -> Option<Value> {
        match self.peek_char() {
            '0'..='9' => self.parse_length(),
            '#' => self.parse_color(),
            '(' => {
                // Anonymous function call (shouldn't happen in valid CSS, but handle gracefully)
                self.parse_function()
            },
            'v' => {
                // Could be var() or a keyword starting with 'v'
                let saved_pos = self.position;
                if let Some(val) = self.parse_var_reference() {
                    // Check if it's actually a var() or just a keyword
                    if matches!(val, Value::Variable(_)) {
                        Some(val)
                    } else {
                        // Was a keyword, restore position and parse as keyword
                        self.position = saved_pos;
                        Some(Value::Keyword(self.parse_identifier()))
                    }
                } else {
                    self.position = saved_pos;
                    Some(Value::Keyword(self.parse_identifier()))
                }
            },
            'a'..='z' | 'A'..='Z' => {
                // Could be a keyword or function name
                let identifier = self.parse_identifier();
                if identifier.is_empty() {
                    return None;
                }
                
                self.consume_whitespace();
                if self.peek_char() == '(' {
                    // It's a function call
                    // We need to parse the function with the name we already consumed
                    // Save the name and parse function
                    let func_name = identifier;
                    if !self.expect_char('(') {
                        return Some(Value::Keyword(func_name));
                    }
                    self.consume_whitespace();

                    let mut args = Vec::new();
                    let mut iterations = 0;
                    const MAX_FUNCTION_ARGS: usize = 100;
                    let mut last_pos = self.position;

                    while self.peek_char() != ')' && !self.eof() && iterations < MAX_FUNCTION_ARGS {
                        iterations += 1;
                        let pos_before = self.position;
                        
                        if let Some(arg) = self.parse_value() {
                            args.push(arg);
                            last_pos = self.position;
                        } else {
                            // If we didn't advance, try to skip problematic content
                            if self.position == pos_before {
                                // Skip until we find ',' or ')' or '/'
                                while !self.eof() && self.peek_char() != ',' && self.peek_char() != ')' && self.peek_char() != '/' {
                                    self.next_char();
                                }
                                // Handle '/' separator (e.g., rgb(59 130 246 / var(--opacity)))
                                if self.peek_char() == '/' {
                                    self.next_char();
                                    self.consume_whitespace();
                                    if let Some(arg) = self.parse_value() {
                                        args.push(arg);
                                    }
                                }
                            }
                        }
                        
                        self.consume_whitespace();
                        
                        if self.peek_char() == ',' {
                            self.next_char();
                            self.consume_whitespace();
                        } else if self.peek_char() == '/' {
                            // Handle '/' separator for alpha channel
                            self.next_char();
                            self.consume_whitespace();
                            if let Some(arg) = self.parse_value() {
                                args.push(arg);
                            }
                            self.consume_whitespace();
                        }
                        
                        // Safety check: if we're stuck, break
                        if self.position == last_pos && iterations > 1 {
                            log::warn!(target: "css", "Function parsing stuck, breaking");
                            break;
                        }
                    }

                    if !self.expect_char(')') {
                        // Missing closing paren - return what we have
                        log::warn!(target: "css", "Function {} missing closing parenthesis", func_name);
                        return Some(Value::Function(func_name, args));
                    }
                    
                    Some(Value::Function(func_name, args))
                } else {
                    // Just a keyword
                    Some(Value::Keyword(identifier))
                }
            },
            '"' | '\'' => {
                // Parse quoted string
                let quote_char = self.next_char();
                let mut value = String::new();
                let mut iterations = 0;
                const MAX_STRING_ITERATIONS: usize = 10000;
                
                while !self.eof() && self.peek_char() != quote_char && iterations < MAX_STRING_ITERATIONS {
                    if self.peek_char() == '\\' && self.position + 1 < self.input.len() {
                        // Handle escape sequences
                        self.next_char(); // consume '\'
                        if !self.eof() {
                            value.push(self.next_char());
                        }
                    } else {
                        value.push(self.next_char());
                    }
                    iterations += 1;
                }
                
                if self.peek_char() == quote_char {
                    self.next_char(); // consume closing quote
                    Some(Value::String(value))
                } else {
                    // Unclosed string, return what we have
                    Some(Value::String(value))
                }
            },
            _ => {
                // Try to parse as identifier/keyword
                let identifier = self.parse_identifier();
                if identifier.is_empty() {
                    None
                } else {
                    Some(Value::Keyword(identifier))
                }
            },
        }
    }

    fn parse_length(&mut self) -> Option<Value> {
        let num = self
            .consume_while(|c| c.is_ascii_digit() || c == '.')
            .parse()
            .ok()?;
        
        // Check for percentage first (it's a single character, not an identifier)
        if self.peek_char() == '%' {
            self.next_char();
            return Some(Value::Length(num, Unit::Percent));
        }
        
        // Otherwise, parse as identifier for other units
        let unit_str = self.parse_identifier().to_lowercase();
        let unit = match unit_str.as_str() {
            "px" => Unit::Px,
            "em" => Unit::Em,
            "rem" => Unit::Rem,
            _ => return None,
        };
        Some(Value::Length(num, unit))
    }

    fn parse_color(&mut self) -> Option<Value> {
        self.next_char(); // consume '#'
        let hex = self.consume_while(|c| c.is_ascii_hexdigit());
        
        // Support both 3-digit (#fff) and 6-digit (#ffffff) hex colors
        if hex.len() == 3 {
            // Expand 3-digit to 6-digit: #fff -> #ffffff
            let r = u8::from_str_radix(&format!("{}{}", &hex[0..1], &hex[0..1]), 16).ok()?;
            let g = u8::from_str_radix(&format!("{}{}", &hex[1..2], &hex[1..2]), 16).ok()?;
            let b = u8::from_str_radix(&format!("{}{}", &hex[2..3], &hex[2..3]), 16).ok()?;
            Some(Value::Color(Color { r, g, b, a: 255 }))
        } else if hex.len() == 6 {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            Some(Value::Color(Color { r, g, b, a: 255 }))
        } else {
            None
        }
    }

    fn parse_function(&mut self) -> Option<Value> {
        // First, try to parse function name if we haven't consumed '(' yet
        let func_name = if self.peek_char() != '(' {
            let name = self.parse_identifier();
            if name.is_empty() {
                return None;
            }
            name
        } else {
            "unknown".to_string()
        };
        
        if !self.expect_char('(') {
            return None;
        }
        self.consume_whitespace();

        let mut args = Vec::new();
        let mut iterations = 0;
        const MAX_FUNCTION_ARGS: usize = 100;
        let mut last_pos = self.position;

        while self.peek_char() != ')' && !self.eof() && iterations < MAX_FUNCTION_ARGS {
            iterations += 1;
            let pos_before = self.position;
            
            if let Some(arg) = self.parse_value() {
                args.push(arg);
                last_pos = self.position;
            } else {
                // If we didn't advance, try to skip problematic content
                if self.position == pos_before {
                    // Skip until we find ',' or ')' or '/'
                    while !self.eof() && self.peek_char() != ',' && self.peek_char() != ')' && self.peek_char() != '/' {
                        self.next_char();
                    }
                    // Handle '/' separator (e.g., rgb(59 130 246 / var(--opacity)))
                    if self.peek_char() == '/' {
                        self.next_char();
                        self.consume_whitespace();
                        if let Some(arg) = self.parse_value() {
                            args.push(arg);
                        }
                    }
                }
            }
            
            self.consume_whitespace();
            
            if self.peek_char() == ',' {
                self.next_char();
                self.consume_whitespace();
            } else if self.peek_char() == '/' {
                // Handle '/' separator for alpha channel
                self.next_char();
                self.consume_whitespace();
                if let Some(arg) = self.parse_value() {
                    args.push(arg);
                }
                self.consume_whitespace();
            }
            
            // Safety check: if we're stuck, break
            if self.position == last_pos && iterations > 1 {
                log::warn!(target: "css", "Function parsing stuck, breaking");
                break;
            }
        }

        if !self.expect_char(')') {
            // Missing closing paren - return what we have
            log::warn!(target: "css", "Function {} missing closing parenthesis", func_name);
            return Some(Value::Function(func_name, args));
        }
        
        Some(Value::Function(func_name, args))
    }

    fn parse_var_reference(&mut self) -> Option<Value> {
        // Check if it's 'var('
        let identifier = self.parse_identifier();
        if identifier.to_lowercase() == "var" {
            if !self.expect_char('(') {
                return None;
            }
            self.consume_whitespace();
            
            // Parse variable name (can start with -- or be any identifier)
            let var_name = if self.peek_char() == '-' {
                self.next_char();
                if self.peek_char() == '-' {
                    self.next_char();
                    format!("--{}", self.parse_identifier())
                } else {
                    // Single dash, treat as part of identifier
                    format!("-{}", self.parse_identifier())
                }
            } else {
                self.parse_identifier()
            };
            
            self.consume_whitespace();
            
            // Optional fallback value
            if self.peek_char() == ',' {
                self.next_char();
                self.consume_whitespace();
                // For now, we'll just skip the fallback
                // TODO: Parse fallback value properly
                while self.peek_char() != ')' && !self.eof() {
                    self.next_char();
                }
            }
            
            if !self.expect_char(')') {
                return None;
            }
            Some(Value::Variable(var_name))
        } else {
            // Not a var(), treat as keyword
            Some(Value::Keyword(identifier))
        }
    }

    fn parse_identifier(&mut self) -> String {
        let mut result = String::new();
        let mut iterations = 0;
        const MAX_ITERATIONS: usize = 1000;
        
        while !self.eof() && iterations < MAX_ITERATIONS {
            let ch = self.peek_char();
            
            // Handle escape sequences
            if ch == '\\' && self.position + 1 < self.input.len() {
                self.next_char(); // consume '\'
                if !self.eof() {
                    let escaped = self.next_char();
                    result.push(escaped);
                    iterations += 1;
                    continue;
                }
            }
            
            // Valid identifier characters
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                result.push(self.next_char());
                iterations += 1;
            } else {
                break;
            }
        }
        
        result
    }

    fn consume_whitespace(&mut self) {
        loop {
            self.consume_while(|c| c.is_whitespace());
            
            // Skip CSS comments /* ... */
            if self.peek_char() == '/' && self.position + 1 < self.input.len() {
                let next_char = self.input.chars().nth(self.position + 1);
                if next_char == Some('*') {
                    // Found comment start, skip until */
                    self.next_char(); // consume '/'
                    self.next_char(); // consume '*'
                    
                    let mut iterations = 0;
                    const MAX_COMMENT_ITERATIONS: usize = 10000;
                    
                    while !self.eof() && iterations < MAX_COMMENT_ITERATIONS {
                        if self.peek_char() == '*' && self.position + 1 < self.input.len() {
                            let next = self.input.chars().nth(self.position + 1);
                            if next == Some('/') {
                                self.next_char(); // consume '*'
                                self.next_char(); // consume '/'
                                break; // Comment ended
                            }
                        }
                        self.next_char();
                        iterations += 1;
                    }
                    
                    if iterations >= MAX_COMMENT_ITERATIONS {
                        log::warn!(target: "css", "Comment parsing exceeded maximum iterations, stopping");
                        break;
                    }
                    continue; // Check for more whitespace/comments
                }
            }
            break; // No more whitespace or comments
        }
    }

    fn consume_while<F>(&mut self, test: F) -> String
    where
        F: Fn(char) -> bool,
    {
        let mut result = String::new();
        let start_pos = self.position;
        let mut iterations = 0;
        const MAX_ITERATIONS: usize = 10000; // Prevent infinite loops

        while !self.eof() && test(self.peek_char()) && iterations < MAX_ITERATIONS {
            result.push(self.next_char());
            iterations += 1;

            // Safety check: if we haven't moved, break to prevent infinite loop
            if self.position == start_pos && iterations > 0 {
                log::warn!(target: "css", "consume_while: position not advancing, breaking to prevent infinite loop");
                break;
            }
        }

        if iterations >= MAX_ITERATIONS {
            log::warn!(target: "css", "consume_while: exceeded maximum iterations, stopping");
        }

        result
    }

    fn next_char(&mut self) -> char {
        if self.eof() {
            return '\0';
        }

        let mut iter = self.input[self.position..].char_indices();
        if let Some((_, cur_char)) = iter.next() {
            if let Some((next_pos, _)) = iter.next() {
                self.position += next_pos;
            } else {
                // End of input
                self.position = self.input.len();
            }
            cur_char
        } else {
            '\0'
        }
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
