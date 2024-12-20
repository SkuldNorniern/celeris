use log::{debug, warn};

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Keywords
    Let,
    Function,
    If,
    Else,
    While,
    Return,
    New,
    
    // Literals
    Number(f64),
    String(String),
    Identifier(String),
    
    // Operators
    Plus,
    Minus,
    Star,
    Slash,
    Equals,
    DoubleEquals,
    Dot,
    Pipe,
    DoublePipe,
    LeftBracket,
    RightBracket,
    
    // Punctuation
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    Semicolon,
    Comma,
    Colon,
    
    EOF,
}

pub fn tokenize(source: &str) -> Vec<Token> {
    debug!(target: "javascript", "Starting tokenization of source: {:?}", source);
    let mut tokens = Vec::new();
    let mut chars = source.chars().peekable();
    
    while let Some(&c) = chars.peek() {
        match c {
            // Skip whitespace
            c if c.is_whitespace() => {
                chars.next();
            },
            
            // String literals
            '"' | '\'' => {
                chars.next(); // consume opening quote
                let quote = c;
                let mut string = String::new();
                
                while let Some(&c) = chars.peek() {
                    if c == quote {
                        chars.next(); // consume closing quote
                        break;
                    } else if c == '\\' {
                        chars.next(); // consume backslash
                        if let Some(&next) = chars.peek() {
                            string.push(match next {
                                'n' => '\n',
                                't' => '\t',
                                'r' => '\r',
                                '\\' => '\\',
                                '"' => '"',
                                '\'' => '\'',
                                _ => next,
                            });
                            chars.next();
                        }
                    } else {
                        string.push(c);
                        chars.next();
                    }
                }
                
                debug!(target: "javascript", "Found string literal: {:?}", string);
                tokens.push(Token::String(string));
            },
            
            // Numbers
            c if c.is_digit(10) => {
                let mut number = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_digit(10) || c == '.' {
                        number.push(chars.next().unwrap());
                    } else {
                        break;
                    }
                }
                let parsed = number.parse().unwrap_or(0.0);
                debug!(target: "javascript", "Found number: {}", parsed);
                tokens.push(Token::Number(parsed));
            },
            
            // Identifiers and keywords
            c if c.is_alphabetic() || c == '_' => {
                let mut ident = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_alphanumeric() || c == '_' {
                        ident.push(chars.next().unwrap());
                    } else {
                        break;
                    }
                }
                
                let token = match ident.as_str() {
                    "let" => Token::Let,
                    "function" => Token::Function,
                    "if" => Token::If,
                    "else" => Token::Else,
                    "while" => Token::While,
                    "return" => Token::Return,
                    "new" => Token::New,
                    _ => Token::Identifier(ident.clone()),
                };
                debug!(target: "javascript", "Found identifier/keyword: {} -> {:?}", ident, token);
                tokens.push(token);
            },
            
            // Operators and punctuation
            '+' | '-' | '*' | '/' | '(' | ')' | '{' | '}' | ';' | ',' | '=' | '.' | '|' | '[' | ']' | ':' => {
                let token = match c {
                    '+' => {
                        chars.next();
                        Token::Plus
                    },
                    '-' => {
                        chars.next();
                        Token::Minus
                    },
                    '*' => {
                        chars.next();
                        Token::Star
                    },
                    '/' => {
                        chars.next();
                        if chars.peek() == Some(&'/') {
                            debug!(target: "javascript", "Found single-line comment");
                            chars.next();
                            while let Some(&c) = chars.peek() {
                                if c == '\n' {
                                    break;
                                }
                                chars.next();
                            }
                            continue;
                        } else if chars.peek() == Some(&'*') {
                            debug!(target: "javascript", "Found multi-line comment");
                            chars.next();
                            while let Some(&c) = chars.peek() {
                                if c == '*' {
                                    chars.next();
                                    if chars.peek() == Some(&'/') {
                                        chars.next();
                                        break;
                                    }
                                } else {
                                    chars.next();
                                }
                            }
                            continue;
                        }
                        Token::Slash
                    },
                    '(' => {
                        chars.next();
                        Token::LeftParen
                    },
                    ')' => {
                        chars.next();
                        Token::RightParen
                    },
                    '{' => {
                        chars.next();
                        Token::LeftBrace
                    },
                    '}' => {
                        chars.next();
                        Token::RightBrace
                    },
                    ';' => {
                        chars.next();
                        Token::Semicolon
                    },
                    ',' => {
                        chars.next();
                        Token::Comma
                    },
                    '=' => {
                        chars.next();
                        if chars.peek() == Some(&'=') {
                            chars.next();
                            Token::DoubleEquals
                        } else {
                            Token::Equals
                        }
                    },
                    '.' => {
                        chars.next();
                        Token::Dot
                    },
                    '|' => {
                        chars.next();
                        if chars.peek() == Some(&'|') {
                            chars.next();
                            Token::DoublePipe
                        } else {
                            Token::Pipe
                        }
                    },
                    '[' => {
                        chars.next();
                        Token::LeftBracket
                    },
                    ']' => {
                        chars.next();
                        Token::RightBracket
                    },
                    ':' => {
                        chars.next();
                        Token::Colon
                    },
                    _ => unreachable!(),
                };
                debug!(target: "javascript", "Found operator/punctuation: {:?}", token);
                tokens.push(token);
            },
            
            // Skip unknown characters
            c => {
                warn!(target: "javascript", "Skipping unknown character: {:?}", c);
                chars.next();
            }
        }
    }
    
    tokens.push(Token::EOF);
    debug!(target: "javascript", "Tokenization complete. Tokens: {:?}", tokens);
    tokens
} 