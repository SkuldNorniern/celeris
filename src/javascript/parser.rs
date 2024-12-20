use super::ast::{Node, BinaryOperator};
use super::tokenizer::{Token, tokenize};
use std::error::Error;
use log::{debug, error};

pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

impl Parser {
    pub fn new(source: &str) -> Self {
        let tokens = tokenize(source);
        Self {
            tokens,
            current: 0,
        }
    }

    fn is_at_end(&self) -> bool {
        self.current >= self.tokens.len() || self.tokens[self.current] == Token::EOF
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }

    fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.current += 1;
        }
        &self.tokens[self.current - 1]
    }

    pub fn parse(&mut self) -> Result<Node, Box<dyn Error>> {
        let mut program = Vec::new();
        
        while !self.is_at_end() {
            program.push(self.parse_statement()?);
        }
        
        Ok(Node::Program(program))
    }

    fn parse_statement(&mut self) -> Result<Node, Box<dyn Error>> {
        debug!(target: "javascript", "Parsing statement, current token: {:?}", self.peek());
        let stmt = match self.peek() {
            Token::Let => self.parse_variable_declaration()?,
            Token::Function => self.parse_function_declaration()?,
            Token::If => self.parse_if_statement()?,
            Token::While => self.parse_while_statement()?,
            _ => {
                // Expression statement
                let expr = self.parse_expression()?;
                
                // Consume optional semicolon
                if matches!(self.peek(), Token::Semicolon) {
                    debug!(target: "javascript", "Consuming semicolon after expression");
                    self.advance();
                }
                
                expr
            }
        };

        debug!(target: "javascript", "Completed parsing statement");
        Ok(stmt)
    }

    fn parse_variable_declaration(&mut self) -> Result<Node, Box<dyn Error>> {
        self.advance(); // consume 'let'
        let name = match self.advance() {
            Token::Identifier(name) => name.clone(),
            token => {
                error!(target: "javascript", "Expected identifier after 'let', found: {:?}", token);
                return Err("Expected identifier after 'let'".into());
            }
        };

        let init = if matches!(self.peek(), Token::Equals) {
            self.advance(); // consume '='
            Some(Box::new(self.parse_expression()?))
        } else {
            None
        };

        // Consume optional semicolon
        if matches!(self.peek(), Token::Semicolon) {
            debug!(target: "javascript", "Consuming semicolon after variable declaration");
            self.advance();
        }

        Ok(Node::VariableDecl { name, init })
    }

    fn parse_expression(&mut self) -> Result<Node, Box<dyn Error>> {
        debug!(target: "javascript", "Parsing expression, current token: {:?}", self.peek());
        if matches!(self.peek(), Token::New) {
            self.parse_new_expression()
        } else {
            self.parse_assignment()
        }
    }

    fn parse_new_expression(&mut self) -> Result<Node, Box<dyn Error>> {
        self.advance(); // consume 'new'
        debug!(target: "javascript", "Parsing new expression");

        let constructor = Box::new(self.parse_primary()?);
        let mut arguments = Vec::new();

        // Parse constructor arguments if present
        if matches!(self.peek(), Token::LeftParen) {
            self.advance(); // consume '('
            
            if !matches!(self.peek(), Token::RightParen) {
                loop {
                    arguments.push(self.parse_expression()?);
                    
                    match self.peek() {
                        Token::RightParen => {
                            self.advance(); // consume ')'
                            break;
                        }
                        Token::Comma => {
                            self.advance(); // consume ','
                        }
                        token => {
                            error!(target: "javascript", "Expected ',' or ')' in constructor arguments, found: {:?}", token);
                            return Err("Expected ',' or ')' in constructor arguments".into());
                        }
                    }
                }
            } else {
                self.advance(); // consume ')'
            }
        }

        Ok(Node::NewExpr {
            constructor,
            arguments,
        })
    }

    fn parse_assignment(&mut self) -> Result<Node, Box<dyn Error>> {
        let expr = self.parse_logical()?;

        if matches!(self.peek(), Token::Equals) {
            debug!(target: "javascript", "Found assignment operator");
            self.advance(); // consume '='
            let value = self.parse_assignment()?;

            // Verify left side is a valid assignment target
            match expr {
                Node::Identifier(_) | Node::MemberExpr { .. } => {
                    return Ok(Node::AssignmentExpr {
                        left: Box::new(expr),
                        right: Box::new(value),
                    });
                }
                _ => {
                    error!(target: "javascript", "Invalid assignment target: {:?}", expr);
                    return Err("Invalid assignment target".into());
                }
            }
        }

        Ok(expr)
    }

    fn parse_logical(&mut self) -> Result<Node, Box<dyn Error>> {
        debug!(target: "javascript", "Parsing logical expression");
        let mut expr = self.parse_equality()?;

        while matches!(self.peek(), Token::DoublePipe) {
            debug!(target: "javascript", "Found logical OR operator");
            self.advance(); // consume operator
            let right = self.parse_equality()?;
            expr = Node::LogicalOr {
                left: Box::new(expr),
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    fn parse_equality(&mut self) -> Result<Node, Box<dyn Error>> {
        let mut expr = self.parse_additive()?;

        while matches!(self.peek(), Token::DoubleEquals) {
            self.advance(); // consume operator
            let right = self.parse_additive()?;
            expr = Node::BinaryOp {
                op: BinaryOperator::Equal,
                left: Box::new(expr),
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    fn parse_additive(&mut self) -> Result<Node, Box<dyn Error>> {
        let mut expr = self.parse_multiplicative()?;

        while matches!(self.peek(), Token::Plus | Token::Minus) {
            let op = match self.advance() {
                Token::Plus => BinaryOperator::Add,
                Token::Minus => BinaryOperator::Subtract,
                _ => unreachable!(),
            };
            let right = self.parse_multiplicative()?;
            expr = Node::BinaryOp {
                op,
                left: Box::new(expr),
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    fn parse_multiplicative(&mut self) -> Result<Node, Box<dyn Error>> {
        let mut expr = self.parse_primary()?;

        while matches!(self.peek(), Token::Star | Token::Slash) {
            let op = match self.advance() {
                Token::Star => BinaryOperator::Multiply,
                Token::Slash => BinaryOperator::Divide,
                _ => unreachable!(),
            };
            let right = self.parse_primary()?;
            expr = Node::BinaryOp {
                op,
                left: Box::new(expr),
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Node, Box<dyn Error>> {
        debug!(target: "javascript", "Parsing primary expression, current token: {:?}", self.peek());
        let mut expr = match self.peek() {
            Token::LeftBrace => {
                debug!(target: "javascript", "Found object literal start");
                self.advance(); // consume '{'
                let mut properties = Vec::new();

                while !matches!(self.peek(), Token::RightBrace | Token::EOF) {
                    // Parse property key
                    let key = match self.peek() {
                        Token::String(s) => {
                            let key = s.clone();
                            self.advance();
                            key
                        },
                        Token::Identifier(name) => {
                            let key = name.clone();
                            self.advance();
                            key
                        },
                        token => {
                            error!(target: "javascript", "Expected string or identifier as object key, found: {:?}", token);
                            return Err("Expected string or identifier as object key".into());
                        }
                    };

                    // Expect colon
                    match self.advance() {
                        Token::Colon => {},
                        token => {
                            error!(target: "javascript", "Expected ':' after property key, found: {:?}", token);
                            return Err("Expected ':' after property key".into());
                        }
                    }

                    // Parse value
                    let value = match self.peek() {
                        Token::String(s) => {
                            let s = s.clone();
                            self.advance();
                            Box::new(Node::String(s))
                        },
                        Token::Number(n) => {
                            let n = *n;
                            self.advance();
                            Box::new(Node::Number(n))
                        },
                        Token::Identifier(name) => {
                            let name = name.clone();
                            self.advance();
                            Box::new(Node::Identifier(name))
                        },
                        Token::LeftBrace => {
                            // Handle nested object literals
                            Box::new(self.parse_primary()?)
                        },
                        Token::LeftBracket => {
                            // Handle array literals
                            Box::new(self.parse_primary()?)
                        },
                        token => {
                            error!(target: "javascript", "Expected value in object literal, found: {:?}", token);
                            return Err("Expected value in object literal".into());
                        }
                    };

                    properties.push((key, value));

                    // Handle comma
                    if matches!(self.peek(), Token::Comma) {
                        self.advance();
                    }
                }

                match self.advance() {
                    Token::RightBrace => Ok::<Node, Box<dyn Error>>(Node::ObjectLiteral(properties)),
                    token => {
                        error!(target: "javascript", "Expected '}}' at end of object literal, found: {:?}", token);
                        Err("Expected '}' at end of object literal".into())
                    }
                }?
            },
            Token::Number(n) => {
                let n = *n;
                self.advance();
                debug!(target: "javascript", "Found number literal: {}", n);
                Ok::<Node, Box<dyn Error>>(Node::Number(n))?
            },
            Token::String(s) => {
                let s = s.clone();
                self.advance();
                debug!(target: "javascript", "Found string literal: {:?}", s);
                Ok::<Node, Box<dyn Error>>(Node::String(s))?
            },
            Token::Identifier(name) => {
                let name = name.clone();
                self.advance();
                debug!(target: "javascript", "Found identifier: {}", name);
                Ok::<Node, Box<dyn Error>>(Node::Identifier(name))?
            },
            Token::LeftBracket => {
                self.advance();
                debug!(target: "javascript", "Found array literal start");
                let mut elements = Vec::new();
                
                // Handle empty array
                if matches!(self.peek(), Token::RightBracket) {
                    self.advance(); // consume ']'
                    Ok::<Node, Box<dyn Error>>(Node::ArrayLiteral(elements))?
                } else {
                    // Parse array elements
                    loop {
                        elements.push(self.parse_expression()?);
                        
                        match self.peek() {
                            Token::RightBracket => {
                                self.advance(); // consume ']'
                                break;
                            }
                            Token::Comma => {
                                self.advance(); // consume ','
                                if matches!(self.peek(), Token::RightBracket) {
                                    // Handle trailing comma
                                    self.advance(); // consume ']'
                                    break;
                                }
                            }
                            token => {
                                error!(target: "javascript", "Expected ',' or ']', found: {:?}", token);
                                return Err("Expected ',' or ']' in array literal".into());
                            }
                        }
                    }
                    
                    debug!(target: "javascript", "Completed array literal with {} elements", elements.len());
                    Ok::<Node, Box<dyn Error>>(Node::ArrayLiteral(elements))?
                }
            },
            Token::LeftParen => {
                self.advance();
                debug!(target: "javascript", "Found opening parenthesis");
                let expr = self.parse_expression()?;
                match self.advance() {
                    Token::RightParen => {
                        debug!(target: "javascript", "Found closing parenthesis");
                        Ok::<Node, Box<dyn Error>>(expr)?
                    },
                    token => {
                        error!(target: "javascript", "Expected closing parenthesis, found: {:?}", token);
                        return Err(format!("Expected closing parenthesis, found: {:?}", token).into());
                    }
                }
            },
            token => {
                error!(target: "javascript", "Unexpected token in primary expression: {:?}", token);
                return Err(format!("Unexpected token in expression: {:?}", token).into());
            }
        };

        // Parse member expressions and function calls with correct precedence
        loop {
            match self.peek() {
                // First handle member access (. and [])
                Token::Dot => {
                    self.advance(); // consume '.'
                    match self.advance() {
                        Token::Identifier(name) => {
                            expr = Node::MemberExpr {
                                object: Box::new(expr),
                                property: Box::new(Node::Identifier(name.clone())),
                                computed: false,
                            };
                        },
                        token => {
                            error!(target: "javascript", "Expected identifier after '.', found: {:?}", token);
                            return Err("Expected identifier after '.'".into());
                        }
                    }
                }
                Token::LeftBracket => {
                    self.advance(); // consume '['
                    let prop = self.parse_expression()?;
                    match self.advance() {
                        Token::RightBracket => {
                            expr = Node::MemberExpr {
                                object: Box::new(expr),
                                property: Box::new(prop),
                                computed: true,
                            };
                        },
                        token => {
                            error!(target: "javascript", "Expected ']', found: {:?}", token);
                            return Err("Expected ']'".into());
                        }
                    }
                }
                // Then handle function calls
                Token::LeftParen => {
                    self.advance(); // consume '('
                    debug!(target: "javascript", "Parsing function call");
                    
                    let mut arguments = Vec::new();
                    
                    if matches!(self.peek(), Token::RightParen) {
                        debug!(target: "javascript", "Found empty argument list");
                        self.advance(); // consume ')'
                    } else {
                        debug!(target: "javascript", "Parsing function arguments");
                        loop {
                            debug!(target: "javascript", "Parsing argument, current token: {:?}", self.peek());
                            arguments.push(self.parse_expression()?);
                            
                            match self.peek() {
                                Token::RightParen => {
                                    debug!(target: "javascript", "Found closing parenthesis after argument");
                                    self.advance(); // consume ')'
                                    break;
                                }
                                Token::Comma => {
                                    debug!(target: "javascript", "Found comma, expecting more arguments");
                                    self.advance(); // consume ','
                                }
                                token => {
                                    error!(target: "javascript", "Expected ',' or ')' after argument, found: {:?}", token);
                                    return Err(format!("Expected ',' or ')' after argument, found: {:?}", token).into());
                                }
                            }
                        }
                    }
                    
                    debug!(target: "javascript", "Completed function call with {} arguments", arguments.len());
                    expr = Node::CallExpr {
                        callee: Box::new(expr),
                        arguments,
                    };
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    fn parse_function_declaration(&mut self) -> Result<Node, Box<dyn Error>> {
        self.advance(); // consume 'function'
        
        let name = match self.advance() {
            Token::Identifier(name) => name.clone(),
            _ => return Err("Expected function name".into()),
        };

        match self.advance() {
            Token::LeftParen => (),
            _ => return Err("Expected '(' after function name".into()),
        }

        let mut params = Vec::new();
        while !matches!(self.peek(), Token::RightParen) {
            match self.advance() {
                Token::Identifier(param) => params.push(param.clone()),
                _ => return Err("Expected parameter name".into()),
            }

            if matches!(self.peek(), Token::Comma) {
                self.advance(); // consume comma
            }
        }
        self.advance(); // consume ')'

        match self.advance() {
            Token::LeftBrace => (),
            _ => return Err("Expected '{' after function parameters".into()),
        }

        let mut body = Vec::new();
        while !matches!(self.peek(), Token::RightBrace) {
            body.push(self.parse_statement()?);
        }
        self.advance(); // consume '}'

        Ok(Node::FunctionDecl {
            name,
            params,
            body,
        })
    }

    fn parse_if_statement(&mut self) -> Result<Node, Box<dyn Error>> {
        self.advance(); // consume 'if'

        match self.advance() {
            Token::LeftParen => (),
            _ => return Err("Expected '(' after 'if'".into()),
        }

        let condition = Box::new(self.parse_expression()?);

        match self.advance() {
            Token::RightParen => (),
            _ => return Err("Expected ')' after if condition".into()),
        }

        let consequent = Box::new(self.parse_statement()?);

        let alternate = if matches!(self.peek(), Token::Else) {
            self.advance(); // consume 'else'
            Some(Box::new(self.parse_statement()?))
        } else {
            None
        };

        Ok(Node::IfStatement {
            condition,
            consequent,
            alternate,
        })
    }

    fn parse_while_statement(&mut self) -> Result<Node, Box<dyn Error>> {
        self.advance(); // consume 'while'

        match self.advance() {
            Token::LeftParen => (),
            _ => return Err("Expected '(' after 'while'".into()),
        }

        let condition = Box::new(self.parse_expression()?);

        match self.advance() {
            Token::RightParen => (),
            _ => return Err("Expected ')' after while condition".into()),
        }

        let body = Box::new(self.parse_statement()?);

        Ok(Node::WhileLoop {
            condition,
            body,
        })
    }
}

pub fn parse(source: &str) -> Result<Node, Box<dyn Error>> {
    let mut parser = Parser::new(source);
    parser.parse()
} 