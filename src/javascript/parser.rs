use super::ast::{Node, BinaryOperator, UnaryOperator};
use super::tokenizer::{Token, tokenize};
use std::error::Error;
use log::{debug, error, trace};

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
        trace!(target: "javascript", "Parsing statement, current token: {:?}", self.peek());
        let stmt = match self.peek() {
            Token::Let | Token::Const | Token::Var => self.parse_variable_declaration()?,
            Token::Function => {
                // Check if this is a function declaration or expression
                // Function declaration: function name() {}
                // Function expression: function() {} or function name() {}
                // At statement level, if next token is not an identifier, it's likely an expression
                self.advance(); // consume 'function'
                if matches!(self.peek(), Token::LeftParen) {
                    // Anonymous function expression at statement level
                    // This is unusual but valid: function() {}; as a statement
                    self.parse_function_expression_after_keyword()?
                } else if let Token::Identifier(_) = self.peek() {
                    // Has a name, parse as declaration
                    // We already consumed 'function', so we need to parse the rest
                    let name = match self.advance() {
                        Token::Identifier(name) => name.clone(),
                        _ => return Err("Expected function name".into()),
                    };
                    self.parse_function_declaration_rest(name)?
                } else {
                    // Unexpected token after 'function' - might be minified code edge case
                    // Try to parse as anonymous function expression if next is '('
                    // Otherwise, try to skip or handle gracefully
                    let next_token = self.peek().clone();
                    debug!(target: "javascript", "Unexpected token after 'function': {:?}, attempting recovery", next_token);
                    
                    // If we see something that might be part of an expression, try to continue
                    // Otherwise, return an error but make it non-fatal
                    match next_token {
                        Token::Star => {
                            // Generator function - not supported yet, skip
                            self.advance(); // consume '*'
                            // Try to continue parsing
                            if matches!(self.peek(), Token::Identifier(_)) {
                                let name = match self.advance() {
                                    Token::Identifier(name) => name.clone(),
                                    _ => return Err("Expected function name after '*'".into()),
                                };
                                self.parse_function_declaration_rest(name)?
                            } else {
                                return Err("Expected function name after '*'".into());
                            }
                        },
                        _ => {
                            // For other unexpected tokens, this is likely invalid syntax
                            // In minified code, this might be a parsing error or edge case
                            // Try to skip gracefully - return undefined or skip this statement
                            debug!(target: "javascript", "Invalid token after 'function': {:?}, skipping", next_token);
                            // Skip the unexpected token and try to continue
                            self.advance(); // consume the unexpected token
                            // Return undefined as a placeholder - need to return Result type
                            return Ok(Node::Undefined);
                        }
                    }
                }
            },
            Token::If => self.parse_if_statement()?,
            Token::While => self.parse_while_statement()?,
            Token::For => self.parse_for_statement()?,
            Token::Return => self.parse_return_statement()?,
            Token::LeftBrace => self.parse_block()?,
            _ => {
                // Expression statement
                let expr = self.parse_expression()?;
                
                // Consume optional semicolon
                if matches!(self.peek(), Token::Semicolon) {
                    trace!(target: "javascript", "Consuming semicolon after expression");
                    self.advance();
                }
                
                expr
            }
        };

        trace!(target: "javascript", "Completed parsing statement");
        Ok(stmt)
    }

    fn parse_variable_declaration(&mut self) -> Result<Node, Box<dyn Error>> {
        self.advance(); // consume 'let', 'const', or 'var'
        let name = match self.peek() {
            Token::Identifier(name) => {
                let name = name.clone();
                self.advance(); // consume identifier
                name
            },
            Token::In | Token::Of => {
                // 'in' or 'of' after variable keyword - likely malformed for loop
                // Return undefined to allow parsing to continue
                debug!(target: "javascript", "Variable declaration: found {:?} instead of identifier, likely malformed for loop", self.peek());
                return Ok(Node::Undefined);
            },
            token => {
                error!(target: "javascript", "Expected identifier after variable keyword, found: {:?}", token);
                return Err("Expected identifier after variable keyword".into());
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
            trace!(target: "javascript", "Consuming semicolon after variable declaration");
            self.advance();
        }

        Ok(Node::VariableDecl { name, init })
    }
    
    fn parse_for_statement(&mut self) -> Result<Node, Box<dyn Error>> {
        self.advance(); // consume 'for'
        
        if !matches!(self.peek(), Token::LeftParen) {
            // Missing opening paren - might be syntax error or edge case in minified code
            // Could be: for await, for...in/of without parens, or malformed code
            debug!(target: "javascript", "For loop: expected '(', found: {:?}, attempting recovery", self.peek());
            
            // Check for 'await' keyword (for await...of)
            if let Token::Identifier(name) = self.peek() {
                if name == "await" {
                    self.advance(); // consume 'await'
                    // Now check for opening paren
                    if matches!(self.peek(), Token::LeftParen) {
                        self.advance(); // consume '('
                    } else {
                        // Still no paren after await - skip and return undefined
                        debug!(target: "javascript", "For loop: no '(' after 'for await', skipping");
                        return Ok(Node::Undefined);
                    }
                } else {
                    // Not 'await', try to skip tokens
                    let mut skipped = 0;
                    while !matches!(self.peek(), Token::LeftParen | Token::Semicolon | Token::RightBrace | Token::EOF) && skipped < 5 {
                        self.advance();
                        skipped += 1;
                    }
                    // Check again for opening paren
                    if !matches!(self.peek(), Token::LeftParen) {
                        // Still no paren - skip this for statement
                        debug!(target: "javascript", "For loop: could not find '(', skipping for statement");
                        return Ok(Node::Undefined);
                    }
                    self.advance(); // consume '('
                }
            } else if matches!(self.peek(), Token::Semicolon | Token::RightBrace | Token::EOF | Token::Comma) {
                // End of statement or separator - return undefined
                return Ok(Node::Undefined);
            } else {
                // Try to skip a few tokens to find opening paren
                let mut skipped = 0;
                while !matches!(self.peek(), Token::LeftParen | Token::Semicolon | Token::RightBrace | Token::EOF) && skipped < 5 {
                    self.advance();
                    skipped += 1;
                }
                // Check again for opening paren
                if !matches!(self.peek(), Token::LeftParen) {
                    // Still no paren - skip this for statement
                    debug!(target: "javascript", "For loop: could not find '(', skipping for statement");
                    return Ok(Node::Undefined);
                }
                self.advance(); // consume '('
            }
        } else {
            self.advance(); // consume '('
        }
        
        // Check for for...in or for...of loops
        // These have the form: for (let/const/var x in/of expr) or for (x in/of expr)
        let has_var_keyword = matches!(self.peek(), Token::Let | Token::Const | Token::Var);
        
        if has_var_keyword {
            // Save position info for potential backtrack
            let var_token = self.advance().clone(); // consume let/const/var
            
            if let Token::Identifier(var_name) = self.peek().clone() {
                let var_name = var_name.clone();
                self.advance(); // consume identifier
                
                // Check for 'in' or 'of'
                match self.peek() {
                    Token::In => {
                        self.advance(); // consume 'in'
                        let object = Box::new(self.parse_expression()?);
                        
                        if !matches!(self.peek(), Token::RightParen) {
                            return Err("Expected ')' after for...in expression".into());
                        }
                        self.advance(); // consume ')'
                        
                        let body = Box::new(self.parse_statement()?);
                        return Ok(Node::ForInLoop { variable: var_name, object, body });
                    }
                    Token::Of => {
                        self.advance(); // consume 'of'
                        let iterable = Box::new(self.parse_expression()?);
                        
                        if !matches!(self.peek(), Token::RightParen) {
                            return Err("Expected ')' after for...of expression".into());
                        }
                        self.advance(); // consume ')'
                        
                        let body = Box::new(self.parse_statement()?);
                        return Ok(Node::ForOfLoop { variable: var_name, iterable, body });
                    }
                    _ => {
                        // It's a regular for loop, we need to continue parsing
                        // We already consumed 'let/const/var' and identifier
                        // Now check for '=' or ';'
                        let init = if matches!(self.peek(), Token::Equals) {
                            self.advance(); // consume '='
                            let init_expr = self.parse_expression()?;
                            Some(Box::new(Node::VariableDecl { 
                                name: var_name, 
                                init: Some(Box::new(init_expr)) 
                            }))
                        } else {
                            Some(Box::new(Node::VariableDecl { 
                                name: var_name, 
                                init: None 
                            }))
                        };
                        
                        return self.parse_for_loop_rest(init);
                    }
                }
            } else {
                return Err("Expected identifier after variable keyword in for loop".into());
            }
        }
        
        // Check for for...in/of without var keyword: for (x in/of expr)
        if let Token::Identifier(var_name) = self.peek().clone() {
            let var_name = var_name.clone();
            self.advance(); // consume identifier
            
            match self.peek() {
                Token::In => {
                    self.advance(); // consume 'in'
                    let object = Box::new(self.parse_expression()?);
                    
                    if !matches!(self.peek(), Token::RightParen) {
                        return Err("Expected ')' after for...in expression".into());
                    }
                    self.advance(); // consume ')'
                    
                    let body = Box::new(self.parse_statement()?);
                    return Ok(Node::ForInLoop { variable: var_name, object, body });
                }
                Token::Of => {
                    self.advance(); // consume 'of'
                    let iterable = Box::new(self.parse_expression()?);
                    
                    if !matches!(self.peek(), Token::RightParen) {
                        return Err("Expected ')' after for...of expression".into());
                    }
                    self.advance(); // consume ')'
                    
                    let body = Box::new(self.parse_statement()?);
                    return Ok(Node::ForOfLoop { variable: var_name, iterable, body });
                }
                _ => {
                    // Regular for loop - we have an expression starting with an identifier
                    // Continue parsing from where we are
                    // The identifier we consumed is part of an expression
                    // We need to handle this case - continue parsing as expression
                    let init_expr = self.parse_expression_starting_with(Node::Identifier(var_name))?;
                    let init = Some(Box::new(init_expr));
                    return self.parse_for_loop_rest(init);
                }
            }
        }
        
        // Regular for loop: for (init; condition; update)
        let init = if matches!(self.peek(), Token::Semicolon) {
            None
        } else {
            let expr = self.parse_expression()?;
            Some(Box::new(expr))
        };
        
        self.parse_for_loop_rest(init)
    }
    
    /// Continue parsing a for loop after the init has been parsed
    fn parse_for_loop_rest(&mut self, init: Option<Box<Node>>) -> Result<Node, Box<dyn Error>> {
        // Consume semicolon after init (if not already consumed)
        if matches!(self.peek(), Token::Semicolon) {
            self.advance();
        }
        
        // Parse condition
        let condition = if matches!(self.peek(), Token::Semicolon) {
            None
        } else {
            Some(Box::new(self.parse_expression()?))
        };
        
        // Must have semicolon after condition, but be lenient for edge cases
        if !matches!(self.peek(), Token::Semicolon) {
            debug!(target: "javascript", "For loop: expected ';' after condition, found: {:?}, attempting recovery", self.peek());
            // Try to recover - if we see ')' it might be the end of the for loop
            if matches!(self.peek(), Token::RightParen) {
                // Missing semicolon but found closing paren - treat condition as empty
                debug!(target: "javascript", "For loop: missing semicolon after condition, but found ')', treating as empty condition");
            } else {
                // Skip unexpected token and try to continue
                self.advance();
            }
        } else {
            self.advance(); // consume ';'
        }
        
        // Parse update - this can be empty, a single expression, or comma-separated expressions
        let update = if matches!(self.peek(), Token::RightParen) {
            None
        } else {
            // Parse update expression
            // Note: The expression parser should handle comma operators automatically
            // since comma has the lowest precedence
            Some(Box::new(self.parse_expression()?))
        };
        
        // Must have closing paren after update, but be lenient
        if !matches!(self.peek(), Token::RightParen) {
            debug!(target: "javascript", "For loop: expected ')' after update, found: {:?}, attempting recovery", self.peek());
            // If we see a semicolon, it might be part of the update expression or a syntax error
            // Try to skip it and look for the closing paren
            if matches!(self.peek(), Token::Semicolon) {
                self.advance(); // consume semicolon
                // Now check for closing paren
                if matches!(self.peek(), Token::RightParen) {
                    self.advance(); // consume ')'
                } else {
                    // Still no closing paren - might be malformed, but try to continue
                    debug!(target: "javascript", "For loop: still no ')' after semicolon, found: {:?}", self.peek());
                }
            } else {
                // For other tokens, try to skip and find the closing paren
                // This handles edge cases in minified code
                debug!(target: "javascript", "For loop: skipping unexpected token {:?} to find ')'", self.peek());
            }
        } else {
            self.advance(); // consume ')'
        }
        
        // Parse body
        let body = Box::new(self.parse_statement()?);
        
        Ok(Node::ForLoop { init, condition, update, body })
    }
    
    /// Continue parsing an expression when we've already consumed the first identifier
    fn parse_expression_starting_with(&mut self, first: Node) -> Result<Node, Box<dyn Error>> {
        // This handles cases like `i = 0` where we've already consumed `i`
        let mut expr = first;
        
        // Check for assignment
        if matches!(self.peek(), Token::Equals) {
            self.advance();
            let right = self.parse_expression()?;
            expr = Node::AssignmentExpr {
                left: Box::new(expr),
                right: Box::new(right),
            };
        }
        
        Ok(expr)
    }
    
    /// Try to parse arrow function parameters (returns None if not an arrow function)
    fn try_parse_arrow_function_params(&mut self) -> Option<(Vec<String>, Node)> {
        // This is a simplified approach - we'll rely on backtracking in the main handler
        // Just return None here to let the main handler work
        None
    }
    
    /// Extract parameter names from an expression (for arrow functions parsed as expressions)
    fn extract_params_from_expr(&self, expr: &Node) -> Vec<String> {
        match expr {
            Node::Identifier(name) => vec![name.clone()],
            // Could extend to handle destructuring, rest params, etc.
            _ => vec![],
        }
    }
    
    fn parse_return_statement(&mut self) -> Result<Node, Box<dyn Error>> {
        self.advance(); // consume 'return'
        
        // Check if there's an expression to return
        let expr = if matches!(self.peek(), Token::Semicolon | Token::RightBrace | Token::EOF) {
            None
        } else {
            Some(Box::new(self.parse_expression()?))
        };
        
        // Consume optional semicolon
        if matches!(self.peek(), Token::Semicolon) {
            self.advance();
        }
        
        Ok(Node::ReturnStatement(expr))
    }
    
    fn parse_block(&mut self) -> Result<Node, Box<dyn Error>> {
        self.advance(); // consume '{'
        
        let mut statements = Vec::new();
        let mut max_iterations = 10000; // Safety limit to prevent infinite loops
        let mut iterations = 0;
        
        while !matches!(self.peek(), Token::RightBrace | Token::EOF) && iterations < max_iterations {
            iterations += 1;
            
            // Try to parse a statement
            match self.parse_statement() {
                Ok(stmt) => {
                    statements.push(stmt);
                },
                Err(e) => {
                    // If parsing fails, try to recover
                    debug!(target: "javascript", "Error parsing statement in block: {}, attempting recovery", e);
                    // Skip tokens until we find a closing brace, semicolon, or another statement start
                    let mut skipped = 0;
                    while !matches!(self.peek(), Token::RightBrace | Token::Semicolon | Token::EOF | Token::Let | Token::Const | Token::Var | Token::Function | Token::If | Token::While | Token::For | Token::Return) 
                        && skipped < 100 {
                        self.advance();
                        skipped += 1;
                    }
                    if matches!(self.peek(), Token::Semicolon) {
                        self.advance(); // consume semicolon
                    }
                    // If we found a closing brace, break
                    if matches!(self.peek(), Token::RightBrace) {
                        break;
                    }
                }
            }
        }
        
        if iterations >= max_iterations {
            debug!(target: "javascript", "Block parsing: reached max iterations, stopping");
        }
        
        // Try to consume closing brace if present
        if matches!(self.peek(), Token::RightBrace) {
            self.advance(); // consume '}'
        } else {
            // No closing brace found - might be syntax error, but continue anyway
            debug!(target: "javascript", "Block parsing: no closing brace found, but continuing");
        }
        
        Ok(Node::Block(statements))
    }

    fn parse_expression(&mut self) -> Result<Node, Box<dyn Error>> {
        trace!(target: "javascript", "Parsing expression, current token: {:?}", self.peek());
        let mut expr = if matches!(self.peek(), Token::New) {
            self.parse_new_expression()?
        } else {
            self.parse_assignment()?
        };
        
        // Handle comma operator: expr1, expr2, ..., exprN (evaluates all, returns last)
        let mut comma_exprs = vec![expr];
        while matches!(self.peek(), Token::Comma) {
            self.advance(); // consume ','
            comma_exprs.push(self.parse_assignment()?); // Parse next expression
        }
        
        // If we have multiple expressions, create a CommaExpr
        if comma_exprs.len() > 1 {
            Ok(Node::CommaExpr(comma_exprs))
        } else {
            Ok(comma_exprs.into_iter().next().unwrap())
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
        let expr = self.parse_ternary()?;

        // Handle simple assignment (=)
        if matches!(self.peek(), Token::Equals) {
            trace!(target: "javascript", "Found assignment operator");
            self.advance(); // consume '='
            let value = self.parse_assignment()?;

            // Verify left side is a valid assignment target
            match &expr {
                Node::Identifier(_) | Node::MemberExpr { .. } => {
                    return Ok(Node::AssignmentExpr {
                        left: Box::new(expr),
                        right: Box::new(value),
                    });
                }
                _ => {
                    // Invalid assignment target - in strict mode this would error,
                    // but for compatibility, just return the right-hand value
                    debug!(target: "javascript", "Invalid assignment target: {:?}, ignoring assignment", expr);
                    return Ok(value);
                }
            }
        }
        
        // Handle compound assignments (+=, -=)
        if matches!(self.peek(), Token::PlusEquals | Token::MinusEquals) {
            let op = match self.peek() {
                Token::PlusEquals => BinaryOperator::Add,
                Token::MinusEquals => BinaryOperator::Subtract,
                _ => unreachable!(),
            };
            trace!(target: "javascript", "Found compound assignment operator: {:?}", op);
            self.advance(); // consume the compound operator
            let value = self.parse_assignment()?;
            
            // x += y is equivalent to x = x + y
            match &expr {
                Node::Identifier(_) | Node::MemberExpr { .. } => {
                    let combined = Node::BinaryOp {
                        op,
                        left: Box::new(expr.clone()),
                        right: Box::new(value),
                    };
                    return Ok(Node::AssignmentExpr {
                        left: Box::new(expr),
                        right: Box::new(combined),
                    });
                }
                _ => {
                    error!(target: "javascript", "Invalid compound assignment target: {:?}", expr);
                    return Err("Invalid compound assignment target".into());
                }
            }
        }

        Ok(expr)
    }

    fn parse_logical(&mut self) -> Result<Node, Box<dyn Error>> {
        trace!(target: "javascript", "Parsing logical expression");
        let mut expr = self.parse_logical_and()?;

        while matches!(self.peek(), Token::DoublePipe) {
            debug!(target: "javascript", "Found logical OR operator");
            self.advance(); // consume operator
            let right = self.parse_logical_and()?;
            expr = Node::BinaryOp {
                op: BinaryOperator::LogicalOr,
                left: Box::new(expr),
                right: Box::new(right),
            };
        }

        Ok(expr)
    }
    
    fn parse_ternary(&mut self) -> Result<Node, Box<dyn Error>> {
        let mut expr = self.parse_logical()?;
        
        // Handle ternary operator: condition ? true_expr : false_expr
        // Ternary is right-associative: a ? b : c ? d : e parses as a ? b : (c ? d : e)
        while matches!(self.peek(), Token::Question) {
            self.advance(); // consume '?'
            
            // Parse true expression - use assignment level to allow all expressions
            let true_expr = self.parse_assignment()?;
            
            match self.peek() {
                Token::Colon => {
                    self.advance(); // consume ':'
                },
                Token::Question => {
                    // Nested ternary without colon - might be: a ? b ? c : d
                    // This is invalid syntax, but try to recover by treating as nested ternary
                    debug!(target: "javascript", "Ternary: found '?' instead of ':', treating as nested ternary");
                    // Continue to parse the nested ternary
                },
                token => {
                    // Missing colon - might be syntax error or the '?' wasn't actually a ternary
                    debug!(target: "javascript", "Ternary: expected ':' after true expression, found: {:?}, attempting recovery", token);
                    // Try to continue - maybe the true expression was actually the whole expression
                    // Return the expression without the ternary
                    return Ok(expr);
                }
            }
            
            // Parse false expression - use assignment level to allow all expressions including nested ternaries
            let false_expr = self.parse_assignment()?;
            
            expr = Node::TernaryExpr {
                condition: Box::new(expr),
                true_expr: Box::new(true_expr),
                false_expr: Box::new(false_expr),
            };
        }
        
        Ok(expr)
    }
    
    fn parse_logical_and(&mut self) -> Result<Node, Box<dyn Error>> {
        let mut expr = self.parse_equality()?;

        while matches!(self.peek(), Token::DoubleAmpersand) {
            self.advance(); // consume operator
            let right = self.parse_equality()?;
            expr = Node::BinaryOp {
                op: BinaryOperator::LogicalAnd,
                left: Box::new(expr),
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    fn parse_equality(&mut self) -> Result<Node, Box<dyn Error>> {
        let mut expr = self.parse_comparison()?;

        while matches!(self.peek(), Token::DoubleEquals | Token::TripleEquals | Token::NotEquals | Token::NotDoubleEquals) {
            let op = match self.advance() {
                Token::DoubleEquals => BinaryOperator::Equal,
                Token::TripleEquals => BinaryOperator::StrictEqual,
                Token::NotEquals => BinaryOperator::NotEqual,
                Token::NotDoubleEquals => BinaryOperator::StrictNotEqual,
                _ => unreachable!(),
            };
            let right = self.parse_comparison()?;
            expr = Node::BinaryOp {
                op,
                left: Box::new(expr),
                right: Box::new(right),
            };
        }

        Ok(expr)
    }
    
    fn parse_comparison(&mut self) -> Result<Node, Box<dyn Error>> {
        let mut expr = self.parse_additive()?;

        while matches!(self.peek(), Token::LessThan | Token::GreaterThan | Token::LessThanEquals | Token::GreaterThanEquals | Token::Instanceof | Token::In) {
            let op = match self.advance() {
                Token::LessThan => BinaryOperator::LessThan,
                Token::GreaterThan => BinaryOperator::GreaterThan,
                Token::LessThanEquals => BinaryOperator::LessThanEqual,
                Token::GreaterThanEquals => BinaryOperator::GreaterThanEqual,
                Token::Instanceof => BinaryOperator::Instanceof,
                Token::In => BinaryOperator::In,
                _ => unreachable!(),
            };
            let right = self.parse_additive()?;
            expr = Node::BinaryOp {
                op,
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
        let mut expr = self.parse_unary()?;

        while matches!(self.peek(), Token::Star | Token::Slash | Token::Percent) {
            let op = match self.advance() {
                Token::Star => BinaryOperator::Multiply,
                Token::Slash => BinaryOperator::Divide,
                Token::Percent => BinaryOperator::Modulo,
                _ => unreachable!(),
            };
            let right = self.parse_unary()?;
            expr = Node::BinaryOp {
                op,
                left: Box::new(expr),
                right: Box::new(right),
            };
        }

        Ok(expr)
    }
    
    fn parse_unary(&mut self) -> Result<Node, Box<dyn Error>> {
        // Handle unary operators: !, -, typeof, ++, --
        match self.peek() {
            Token::Bang => {
                self.advance();
                let operand = self.parse_unary()?;
                Ok(Node::UnaryOp {
                    op: UnaryOperator::Not,
                    operand: Box::new(operand),
                })
            }
            Token::Minus => {
                self.advance();
                let operand = self.parse_unary()?;
                Ok(Node::UnaryOp {
                    op: UnaryOperator::Negative,
                    operand: Box::new(operand),
                })
            }
            Token::Typeof => {
                self.advance();
                let operand = self.parse_unary()?;
                Ok(Node::UnaryOp {
                    op: UnaryOperator::Typeof,
                    operand: Box::new(operand),
                })
            }
            Token::PlusPlus => {
                self.advance();
                let operand = self.parse_unary()?;
                Ok(Node::UnaryOp {
                    op: UnaryOperator::PreIncrement,
                    operand: Box::new(operand),
                })
            }
            Token::MinusMinus => {
                self.advance();
                let operand = self.parse_unary()?;
                Ok(Node::UnaryOp {
                    op: UnaryOperator::PreDecrement,
                    operand: Box::new(operand),
                })
            }
            _ => self.parse_call_or_member(),
        }
    }
    
    fn parse_call_or_member(&mut self) -> Result<Node, Box<dyn Error>> {
        let mut expr = self.parse_primary()?;
        
        // Handle member access and function calls
        loop {
            match self.peek() {
                Token::Dot => {
                    self.advance();
                    if let Token::Identifier(name) = self.advance().clone() {
                        expr = Node::MemberExpr {
                            object: Box::new(expr),
                            property: Box::new(Node::Identifier(name)),
                            computed: false,
                        };
                    } else {
                        return Err("Expected identifier after '.'".into());
                    }
                }
                Token::LeftBracket => {
                    self.advance();
                    let property = self.parse_expression()?;
                    if !matches!(self.peek(), Token::RightBracket) {
                        return Err("Expected ']' after computed property".into());
                    }
                    self.advance();
                    expr = Node::MemberExpr {
                        object: Box::new(expr),
                        property: Box::new(property),
                        computed: true,
                    };
                }
                Token::LeftParen => {
                    self.advance();
                    let mut arguments = Vec::new();
                    
                    if !matches!(self.peek(), Token::RightParen) {
                        loop {
                            arguments.push(self.parse_expression()?);
                            match self.peek() {
                                Token::Comma => { self.advance(); }
                                Token::RightParen => break,
                                _ => return Err("Expected ',' or ')' in arguments".into()),
                            }
                        }
                    }
                    self.advance(); // consume ')'
                    
                    expr = Node::CallExpr {
                        callee: Box::new(expr),
                        arguments,
                    };
                }
                Token::PlusPlus => {
                    // Post-increment: i++
                    self.advance();
                    expr = Node::UnaryOp {
                        op: UnaryOperator::PostIncrement,
                        operand: Box::new(expr),
                    };
                }
                Token::MinusMinus => {
                    // Post-decrement: i--
                    self.advance();
                    expr = Node::UnaryOp {
                        op: UnaryOperator::PostDecrement,
                        operand: Box::new(expr),
                    };
                }
                _ => break,
            }
        }
        
        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Node, Box<dyn Error>> {
        trace!(target: "javascript", "Parsing primary expression, current token: {:?}", self.peek());
        let mut expr = match self.peek() {
            Token::LeftBrace => {
                debug!(target: "javascript", "Found object literal start");
                self.advance(); // consume '{'
                let mut properties = Vec::new();

                while !matches!(self.peek(), Token::RightBrace | Token::EOF) {
                    // Parse property key - can be identifier, string, or computed [expr]
                    let (key, is_computed) = match self.peek() {
                        Token::String(s) => {
                            let key = s.clone();
                            self.advance();
                            (key, false)
                        },
                        Token::Identifier(name) => {
                            let key = name.clone();
                            self.advance();
                            // Check for ES6 shorthand property: { key } is equivalent to { key: key }
                            if matches!(self.peek(), Token::Comma | Token::RightBrace) {
                                // Shorthand property - use the identifier as both key and value
                                let value = Box::new(Node::Identifier(key.clone()));
                                properties.push((key, value));
                                
                                // Handle comma
                                if matches!(self.peek(), Token::Comma) {
                                    self.advance();
                                }
                                continue;
                            }
                            // Check for method shorthand: { method() {} } or { method: function() {} }
                            if matches!(self.peek(), Token::LeftParen) {
                                // Could be method shorthand - but we need to see if it's followed by { or :
                                // For now, treat as regular property key
                            }
                            (key, false)
                        },
                        Token::Function => {
                            // Method shorthand: { function name() {} } or anonymous function as value
                            // This is tricky - could be method shorthand or function as value
                            // For now, parse as function expression and use a generated key
                            debug!(target: "javascript", "Object literal: found 'function' keyword, parsing as function expression");
                            let func_expr = self.parse_function_expression()?;
                            // Use a generated key or try to extract name
                            let key = match &func_expr {
                                Node::FunctionExpr { name: Some(n), .. } => n.clone(),
                                _ => format!("__func_{}", properties.len()),
                            };
                            properties.push((key, Box::new(func_expr)));
                            
                            // Handle comma
                            if matches!(self.peek(), Token::Comma) {
                                self.advance();
                            }
                            continue;
                        },
                        Token::This => {
                            let key = "this".to_string();
                            self.advance();
                            // Check for ES6 shorthand property
                            if matches!(self.peek(), Token::Comma | Token::RightBrace) {
                                let value = Box::new(Node::This);
                                properties.push((key, value));
                                if matches!(self.peek(), Token::Comma) {
                                    self.advance();
                                }
                                continue;
                            }
                            (key, false)
                        },
                        Token::LeftBracket => {
                            // Computed property name: [expression]
                            self.advance(); // consume '['
                            let key_expr = self.parse_expression()?;
                            if !matches!(self.peek(), Token::RightBracket) {
                                return Err("Expected ']' after computed property name".into());
                            }
                            self.advance(); // consume ']'
                            
                            // Extract key string from expression (simplified)
                            let key_str = match &key_expr {
                                Node::String(s) => s.clone(),
                                Node::Identifier(s) => s.clone(),
                                _ => format!("{:?}", key_expr), // Fallback
                            };
                            (key_str, true)
                        },
                        token => {
                            // Try to parse as an expression (might be a member expression or other complex key)
                            // This handles cases like invalid syntax more gracefully
                            debug!(target: "javascript", "Object key: unexpected token {:?}, trying to parse as expression", token);
                            let key_expr = self.parse_expression()?;
                            
                            // Extract key string from expression
                            let key_str = match &key_expr {
                                Node::String(s) => s.clone(),
                                Node::Identifier(s) => s.clone(),
                                Node::MemberExpr { object, property, .. } => {
                                    // For member expressions like obj.prop, use a string representation
                                    format!("{:?}", key_expr)
                                }
                                _ => format!("{:?}", key_expr),
                            };
                            (key_str, true) // Treat as computed
                        }
                    };

                    // Expect colon (unless it's a method shorthand which we already handled)
                    match self.peek() {
                        Token::Colon => {
                            self.advance(); // consume ':'
                        },
                        Token::Comma | Token::RightBrace => {
                            // Missing colon - treat as shorthand property
                            debug!(target: "javascript", "Object literal: missing ':' after key, treating as shorthand property");
                            let value = if key == "this" {
                                Box::new(Node::This)
                            } else {
                                Box::new(Node::Identifier(key.clone()))
                            };
                            properties.push((key, value));
                            
                            // Handle comma
                            if matches!(self.peek(), Token::Comma) {
                                self.advance();
                            }
                            continue;
                        },
                        Token::This | Token::Identifier(_) | Token::String(_) | Token::Number(_) | Token::True | Token::False | Token::Null | Token::LeftBrace | Token::LeftBracket => {
                            // Missing colon but we have a value - treat as shorthand property with the next token as the value
                            // This handles cases like { key value } which should be { key: value }
                            debug!(target: "javascript", "Object literal: missing ':' after key '{}', treating next token as value", key);
                            // Parse the value
                            let value = self.parse_expression()?;
                            properties.push((key, Box::new(value)));
                            
                            // Handle comma
                            if matches!(self.peek(), Token::Comma) {
                                self.advance();
                            }
                            continue;
                        },
                        Token::Function | Token::Bang | Token::Ampersand | Token::Pipe => {
                            // Operators or function keyword - try to parse as expression value
                            let token_type = format!("{:?}", self.peek());
                            trace!(target: "javascript", "Object literal: found {} after key, parsing as expression value", token_type);
                            let value = self.parse_expression()?;
                            properties.push((key, Box::new(value)));
                            
                            // Handle comma
                            if matches!(self.peek(), Token::Comma) {
                                self.advance();
                            }
                            continue;
                        },
                        Token::Return | Token::If | Token::While | Token::For | Token::Let | Token::Const | Token::Var | Token::Function => {
                            // Statement-level tokens found in object literal - likely syntax error or object boundary issue
                            // Exit object literal parsing gracefully
                            let token_type = format!("{:?}", self.peek());
                            trace!(target: "javascript", "Object literal: found statement token {}, ending object literal", token_type);
                            break;
                        },
                        token => {
                            // For other unexpected tokens, try to be lenient
                            debug!(target: "javascript", "Object literal: expected ':' after property key, found {:?}, attempting recovery", token);
                            // Try to skip this property and continue, or exit
                            if matches!(self.peek(), Token::RightBrace | Token::Comma) {
                                // Might be end of object or next property
                                break;
                            }
                            // Skip the unexpected token and try to continue
                            self.advance();
                            continue;
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
                        Token::True => {
                            self.advance();
                            Box::new(Node::Boolean(true))
                        },
                        Token::False => {
                            self.advance();
                            Box::new(Node::Boolean(false))
                        },
                        Token::Null => {
                            self.advance();
                            Box::new(Node::Null)
                        },
                        Token::Undefined => {
                            self.advance();
                            Box::new(Node::Undefined)
                        },
                        Token::Identifier(name) => {
                            let name = name.clone();
                            self.advance();
                            Box::new(Node::Identifier(name))
                        },
                        Token::Function => {
                            // Function expression as object property value
                            Box::new(self.parse_function_expression()?)
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
                            // Try parsing as a general expression (for things like function calls, ternary, etc.)
                            debug!(target: "javascript", "Object literal value: trying to parse as expression, found: {:?}", token);
                            Box::new(self.parse_expression()?)
                        }
                    };

                    properties.push((key, value));

                    // Handle comma
                    if matches!(self.peek(), Token::Comma) {
                        self.advance();
                    }
                }

                // Try to consume closing brace
                if matches!(self.peek(), Token::RightBrace) {
                    self.advance(); // consume '}'
                } else {
                    // No closing brace found - might be syntax error, but return what we have
                    debug!(target: "javascript", "Object literal: expected '}}', found {:?}, ending object literal", self.peek());
                }
                
                Ok::<Node, Box<dyn Error>>(Node::ObjectLiteral(properties))?
            },
            Token::Number(n) => {
                let n = *n;
                self.advance();
                trace!(target: "javascript", "Found number literal: {}", n);
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
                trace!(target: "javascript", "Found identifier: {}", name);
                Ok::<Node, Box<dyn Error>>(Node::Identifier(name))?
            },
            Token::True => {
                self.advance();
                Ok::<Node, Box<dyn Error>>(Node::Boolean(true))?
            },
            Token::False => {
                self.advance();
                Ok::<Node, Box<dyn Error>>(Node::Boolean(false))?
            },
            Token::Null => {
                self.advance();
                Ok::<Node, Box<dyn Error>>(Node::Null)?
            },
            Token::This => {
                self.advance();
                Ok::<Node, Box<dyn Error>>(Node::This)?
            },
            Token::Undefined => {
                self.advance();
                Ok::<Node, Box<dyn Error>>(Node::Undefined)?
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
                                // Allow other tokens - might be part of a complex expression
                                // The expression parser should handle it, but if we're here, 
                                // the expression parsing might have consumed too much
                                // Try to continue if we see a closing brace (might be object in array)
                                if matches!(token, Token::RightBrace) {
                                    // This might be an object literal inside the array
                                    // The expression parser should have handled it, but if we're here,
                                    // it might be a syntax error. Let's be lenient and try to continue.
                                    debug!(target: "javascript", "Array literal: found '}}' unexpectedly, might be malformed expression");
                                    // Don't advance, let the expression parser or outer parser handle it
                                    break;
                                }
                                // For identifiers or other tokens, might be missing comma or syntax error
                                // Try to be lenient - treat as next element (missing comma) or end array
                                debug!(target: "javascript", "Array literal: expected ',' or ']', found {:?}, attempting recovery", token);
                                // If it looks like it could be the start of a new expression, treat as missing comma
                                if matches!(token, Token::Identifier(_) | Token::String(_) | Token::Number(_) | Token::True | Token::False | Token::Null | Token::LeftBrace | Token::LeftBracket | Token::LeftParen) {
                                    // Might be next element with missing comma - continue parsing
                                    continue;
                                } else {
                                    // For other tokens, end the array
                                    break;
                                }
                            }
                        }
                    }
                    
                    trace!(target: "javascript", "Completed array literal with {} elements", elements.len());
                    Ok::<Node, Box<dyn Error>>(Node::ArrayLiteral(elements))?
                }
            },
            Token::LeftParen => {
                self.advance();
                debug!(target: "javascript", "Found opening parenthesis");
                
                // Try to parse as arrow function parameters
                let arrow_result = self.try_parse_arrow_function_params();
                
                if let Some((params, body)) = arrow_result {
                    Ok::<Node, Box<dyn Error>>(Node::ArrowFunction {
                        params,
                        body: Box::new(body),
                    })?
                } else {
                    // Handle empty parens ()
                    if matches!(self.peek(), Token::RightParen) {
                        self.advance();
                        // Check for arrow
                        if matches!(self.peek(), Token::Arrow) {
                            self.advance(); // consume =>
                            let body = if matches!(self.peek(), Token::LeftBrace) {
                                self.parse_block()?
                            } else {
                                self.parse_expression()?
                            };
                            return Ok(Node::ArrowFunction {
                                params: vec![],
                                body: Box::new(body),
                            });
                        }
                        // Empty parens without arrow - undefined
                        Ok::<Node, Box<dyn Error>>(Node::Undefined)?
                    } else {
                        let mut expr = self.parse_expression()?;
                        
                        // Handle comma operator: (expr1, expr2, ..., exprN) returns exprN
                        while matches!(self.peek(), Token::Comma) {
                            self.advance(); // consume comma
                            // Parse next expression (comma operator evaluates all and returns last)
                            expr = self.parse_expression()?;
                        }
                        
                        match self.peek() {
                            Token::RightParen => {
                                self.advance(); // consume ')'
                                debug!(target: "javascript", "Found closing parenthesis");
                                // Check for arrow after )
                                if matches!(self.peek(), Token::Arrow) {
                                    // This was an arrow function params list
                                    // We already parsed it as expression, so extract params if possible
                                    self.advance(); // consume =>
                                    let body = if matches!(self.peek(), Token::LeftBrace) {
                                        self.parse_block()?
                                    } else {
                                        self.parse_expression()?
                                    };
                                    // Try to extract param name from expr
                                    let params = self.extract_params_from_expr(&expr);
                                    return Ok(Node::ArrowFunction {
                                        params,
                                        body: Box::new(body),
                                    });
                                }
                                Ok::<Node, Box<dyn Error>>(expr)?
                            },
                            Token::Ampersand | Token::Pipe => {
                                // Bitwise operators - the expression might continue, but for parenthesized expr, return what we have
                                debug!(target: "javascript", "Parenthesized expression: found bitwise operator, returning parsed expression");
                                Ok::<Node, Box<dyn Error>>(expr)?
                            },
                            token => {
                                // Expression might have consumed the closing paren, or there's a syntax issue
                                // Try to be lenient - return the expression we parsed
                                debug!(target: "javascript", "Parenthesized expression: expected ')', found {:?}, returning parsed expression", token);
                                Ok::<Node, Box<dyn Error>>(expr)?
                            }
                        }
                    }
                }
            },
            Token::Function => {
                // Function expression (anonymous function)
                self.parse_function_expression()?
            },
            token => {
                // For unsupported tokens, return undefined and skip to next statement
                debug!(target: "javascript", "Unsupported token in expression: {:?}, treating as undefined", token);
                self.advance();
                Ok::<Node, Box<dyn Error>>(Node::Undefined)?
            }
        };

        // Parse member expressions and function calls with correct precedence
        loop {
            match self.peek() {
                // First handle member access (. and [])
                Token::Dot => {
                    self.advance(); // consume '.'
                    match self.peek() {
                        Token::Identifier(name) => {
                            let name = name.clone();
                            self.advance(); // consume identifier
                            expr = Node::MemberExpr {
                                object: Box::new(expr),
                                property: Box::new(Node::Identifier(name)),
                                computed: false,
                            };
                        },
                        Token::Star | Token::LeftBracket => {
                            // Spread operator or computed property - skip for now
                            debug!(target: "javascript", "Skipping unsupported syntax after '.': {:?}", self.peek());
                            break;
                        },
                        token => {
                            // Unexpected token after '.' - might be syntax error or unsupported syntax
                            debug!(target: "javascript", "Unexpected token after '.': {:?}, stopping member access parsing", token);
                            break;
                        }
                    }
                }
                Token::LeftBracket => {
                    self.advance(); // consume '['
                    let prop = self.parse_expression()?;
                    match self.peek() {
                        Token::RightBracket => {
                            self.advance(); // consume ']'
                            expr = Node::MemberExpr {
                                object: Box::new(expr),
                                property: Box::new(prop),
                                computed: true,
                            };
                        },
                        Token::Semicolon | Token::Comma | Token::RightParen | Token::RightBrace | Token::Identifier(_) | Token::Dot | Token::LeftParen => {
                            // Unexpected token - might be syntax error, but try to recover
                            debug!(target: "javascript", "Computed property: expected ']', found {:?}, attempting recovery", self.peek());
                            // Return the member expression anyway - the property might have been fully parsed
                            // The expression parser might have consumed too much
                            expr = Node::MemberExpr {
                                object: Box::new(expr),
                                property: Box::new(prop),
                                computed: true,
                            };
                        },
                        token => {
                            // For other tokens, still try to recover
                            debug!(target: "javascript", "Computed property: expected ']', found {:?}, creating member expr anyway", token);
                            expr = Node::MemberExpr {
                                object: Box::new(expr),
                                property: Box::new(prop),
                                computed: true,
                            };
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
                            // Use parse_assignment() instead of parse_expression() to avoid comma operator
                            // The comma between arguments is handled by this loop, not by the expression parser
                            let arg = self.parse_assignment()?;
                            
                            // Check if the next token might be part of a member access that wasn't fully parsed
                            // This can happen with minified code or complex expressions
                            let mut final_arg = arg;
                            loop {
                                let next_token = self.peek().clone();
                                match next_token {
                                    Token::Dot => {
                                        self.advance(); // consume '.'
                                        let next_after_dot = self.peek().clone();
                                        match next_after_dot {
                                            Token::Identifier(name) => {
                                                self.advance(); // consume identifier
                                                final_arg = Node::MemberExpr {
                                                    object: Box::new(final_arg),
                                                    property: Box::new(Node::Identifier(name)),
                                                    computed: false,
                                                };
                                            },
                                            Token::Star | Token::LeftBracket => {
                                                // Spread operator or computed property - stop parsing
                                                break;
                                            },
                                            _ => break,
                                        }
                                    },
                                    Token::LeftBracket => {
                                        self.advance(); // consume '['
                                        let prop = self.parse_expression()?;
                                        if matches!(self.peek(), Token::RightBracket) {
                                            self.advance(); // consume ']'
                                            final_arg = Node::MemberExpr {
                                                object: Box::new(final_arg),
                                                property: Box::new(prop),
                                                computed: true,
                                            };
                                        } else {
                                            break;
                                        }
                                    },
                                    _ => break,
                                }
                            }
                            
                            arguments.push(final_arg);
                            
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
                                Token::Identifier(_) | Token::Dot => {
                                    // Might be a syntax error, but try to continue
                                    // Could be missing comma or part of member access
                                    debug!(target: "javascript", "Unexpected token after argument: {:?}, might be syntax error", self.peek());
                                    // Try to parse as next argument (missing comma)
                                    continue;
                                }
                                Token::Semicolon => {
                                    // Semicolon in argument list - likely syntax error, but try to end the call
                                    debug!(target: "javascript", "Found semicolon in function call arguments, ending argument list");
                                    break;
                                }
                                token => {
                                    // For other unexpected tokens, try to be lenient
                                    debug!(target: "javascript", "Unexpected token after argument: {:?}, ending argument list", token);
                                    break;
                                }
                            }
                        }
                    }
                    
                    log::trace!(target: "javascript", "Parser: Completed function call with {} arguments", arguments.len());
                    for (i, arg) in arguments.iter().enumerate() {
                        let is_func = matches!(arg, Node::FunctionExpr { .. });
                        log::trace!(target: "javascript", "Parser: Argument {} is FunctionExpr: {}", i, is_func);
                    }
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
        
        self.parse_function_declaration_rest(name)
    }
    
    fn parse_function_declaration_rest(&mut self, name: String) -> Result<Node, Box<dyn Error>> {
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
    
    fn parse_function_expression_after_keyword(&mut self) -> Result<Node, Box<dyn Error>> {
        // We've already consumed 'function', now parse the rest as an expression
        // Optional function name (for named function expressions)
        let name = if let Token::Identifier(name) = self.peek() {
            let n = name.clone();
            self.advance();
            Some(n)
        } else {
            None
        };

        if !matches!(self.peek(), Token::LeftParen) {
            return Err("Expected '(' after function".into());
        }
        self.advance(); // consume '('

        let mut params = Vec::new();
        while !matches!(self.peek(), Token::RightParen | Token::EOF) {
            match self.advance() {
                Token::Identifier(param) => params.push(param.clone()),
                Token::Comma => continue,
                Token::RightParen => break,
                Token::EOF => break,
                _ => return Err("Expected parameter name".into()),
            }

            if matches!(self.peek(), Token::Comma) {
                self.advance(); // consume comma
            }
        }
        
        if matches!(self.peek(), Token::RightParen) {
            self.advance(); // consume ')'
        }

        if !matches!(self.peek(), Token::LeftBrace) {
            return Err("Expected '{' after function parameters".into());
        }
        self.advance(); // consume '{'

        let mut body = Vec::new();
        while !matches!(self.peek(), Token::RightBrace | Token::EOF) {
            body.push(self.parse_statement()?);
        }
        
        if matches!(self.peek(), Token::RightBrace) {
            self.advance(); // consume '}'
        }

        Ok(Node::FunctionExpr {
            name,
            params,
            body,
        })
    }
    
    fn parse_function_expression(&mut self) -> Result<Node, Box<dyn Error>> {
        self.advance(); // consume 'function'
        self.parse_function_expression_after_keyword()
    }

    fn parse_if_statement(&mut self) -> Result<Node, Box<dyn Error>> {
        self.advance(); // consume 'if'

        match self.advance() {
            Token::LeftParen => (),
            _ => return Err("Expected '(' after 'if'".into()),
        }

        let condition = Box::new(self.parse_expression()?);

        // Try to find closing parenthesis - be lenient if condition consumed too much
        match self.peek() {
            Token::RightParen => {
                self.advance(); // consume ')'
            },
            token => {
                // Condition might have consumed the closing paren, or there's a syntax error
                // Try to continue parsing - the statement parser will handle errors
                debug!(target: "javascript", "If statement: expected ')', found {:?}, attempting to continue", token);
                // Don't error here - let the statement parser handle it
            }
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