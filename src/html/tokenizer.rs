use crate::dom::Attribute;

#[derive(Debug, PartialEq)]
pub enum Token {
    StartTag {
        name: String,
        attributes: Vec<Attribute>,
    },
    EndTag {
        name: String,
    },
    Text(String),
    Comment(String),
    Doctype(String),
}

pub struct Tokenizer {
    input: Vec<char>,
    position: usize,
    pending_text: Option<String>,
}

impl Tokenizer {
    pub fn new(input: String) -> Self {
        Self {
            input: input.chars().collect(),
            position: 0,
            pending_text: None,
        }
    }

    pub fn next_token(&mut self) -> Option<Token> {
        // If we have pending text from a script/style tag, return it first
        if let Some(text) = self.pending_text.take() {
            return Some(Token::Text(text));
        }

        self.consume_whitespace();

        if self.position >= self.input.len() {
            return None;
        }

        match self.current_char() {
            '<' => {
                self.position += 1;
                match self.current_char() {
                    '!' => self.consume_comment_or_doctype(),
                    '/' => self.consume_end_tag(),
                    _ => self.consume_start_tag(),
                }
            }
            _ => self.consume_text(),
        }
    }

    fn consume_start_tag(&mut self) -> Option<Token> {
        let mut name = String::new();
        let mut attributes = Vec::new();

        // Parse tag name
        while !self.eof() && !self.current_char().is_whitespace() && self.current_char() != '>' && self.current_char() != '/' {
            name.push(self.consume_char());
        }

        // Parse attributes
        self.consume_whitespace();
        while !self.eof() && self.current_char() != '>' {
            // Handle self-closing tag marker (/>)
            if self.current_char() == '/' {
                self.position += 1;
                // Skip to '>' 
                self.consume_whitespace();
                break;
            }
            
            if let Some(attr) = self.consume_attribute() {
                attributes.push(attr);
            }
            self.consume_whitespace();
        }

        // Consume '>'
        if !self.eof() && self.current_char() == '>' {
            self.position += 1;
        }

        // For script/style tags, capture their content as raw text
        let tag_lower = name.to_lowercase();
        if tag_lower == "script" || tag_lower == "style" {
            if let Some(text_content) = self.consume_raw_text(&tag_lower) {
                // Store it to be returned as the next token
                self.pending_text = Some(text_content);
            }
        }

        Some(Token::StartTag { name, attributes })
    }

    fn consume_end_tag(&mut self) -> Option<Token> {
        self.position += 1; // Consume '/'
        let mut name = String::new();

        while !self.eof() && self.current_char() != '>' {
            name.push(self.consume_char());
        }

        // Consume '>'
        if !self.eof() {
            self.position += 1;
        }

        Some(Token::EndTag { name })
    }

    fn consume_text(&mut self) -> Option<Token> {
        let mut text = String::new();

        while !self.eof() && self.current_char() != '<' {
            text.push(self.consume_char());
        }

        if text.is_empty() {
            None
        } else {
            Some(Token::Text(text))
        }
    }

    fn consume_attribute(&mut self) -> Option<Attribute> {
        let mut name = String::new();

        // Parse attribute name (stop at '/' for self-closing tags)
        while !self.eof()
            && !self.current_char().is_whitespace()
            && self.current_char() != '='
            && self.current_char() != '>'
            && self.current_char() != '/'
        {
            name.push(self.consume_char());
        }

        // Empty name means we hit '/' or '>' immediately
        if name.is_empty() {
            return None;
        }

        self.consume_whitespace();

        // Check for '='
        if self.current_char() != '=' {
            return Some(Attribute {
                name,
                value: String::new(),
            });
        }

        self.position += 1; // Consume '='
        self.consume_whitespace();

        // Parse attribute value
        let value = if self.current_char() == '"' || self.current_char() == '\'' {
            let quote = self.consume_char();
            let mut value = String::new();
            while !self.eof() && self.current_char() != quote {
                value.push(self.consume_char());
            }
            if !self.eof() {
                self.position += 1; // Consume closing quote
            }
            value
        } else {
            let mut value = String::new();
            while !self.eof() && !self.current_char().is_whitespace() && self.current_char() != '>' && self.current_char() != '/'
            {
                value.push(self.consume_char());
            }
            value
        };

        Some(Attribute { name, value })
    }

    fn consume_comment_or_doctype(&mut self) -> Option<Token> {
        self.position += 1; // Consume '!'
        if self.current_char() == '-' && self.peek_char() == '-' {
            self.position += 2; // Consume '--'
            self.consume_comment()
        } else if self.current_char() == 'D' || self.current_char() == 'd' {
            self.consume_doctype()
        } else {
            None
        }
    }

    fn consume_comment(&mut self) -> Option<Token> {
        let mut content = String::new();

        while !self.eof() {
            if self.current_char() == '-' && self.peek_char() == '-' && self.peek_next_char() == '>'
            {
                self.position += 3; // Consume '-->'
                break;
            }
            content.push(self.consume_char());
        }

        Some(Token::Comment(content))
    }

    fn consume_doctype(&mut self) -> Option<Token> {
        let mut content = String::new();

        while !self.eof() && self.current_char() != '>' {
            content.push(self.consume_char());
        }

        if !self.eof() {
            self.position += 1; // Consume '>'
        }

        Some(Token::Doctype(content))
    }

    fn consume_whitespace(&mut self) {
        while !self.eof() && self.current_char().is_whitespace() {
            self.position += 1;
        }
    }

    fn current_char(&self) -> char {
        self.input.get(self.position).copied().unwrap_or('\0')
    }

    fn peek_char(&self) -> char {
        self.input.get(self.position + 1).copied().unwrap_or('\0')
    }

    fn peek_next_char(&self) -> char {
        self.input.get(self.position + 2).copied().unwrap_or('\0')
    }

    fn consume_char(&mut self) -> char {
        let c = self.current_char();
        self.position += 1;
        c
    }

    fn eof(&self) -> bool {
        self.position >= self.input.len()
    }

    // Consume raw text content for <script> and <style> tags until their closing tag
    fn consume_raw_text(&mut self, tag_name: &str) -> Option<String> {
        let mut content = String::new();
        let closing_tag = format!("</{}>", tag_name);
        
        while !self.eof() {
            // Check if we're at the closing tag
            if self.position + closing_tag.len() <= self.input.len() {
                let remaining: String = self.input[self.position..self.position + closing_tag.len()].iter().collect();
                if remaining.to_lowercase() == closing_tag.to_lowercase() {
                    // Don't consume the closing tag, let the normal flow handle it
                    break;
                }
            }
            
            content.push(self.consume_char());
        }

        if content.is_empty() {
            None
        } else {
            Some(content)
        }
    }
}
