#[derive(Debug, Clone)]
pub enum Node {
    // Literals
    Number(f64),
    String(String),
    Boolean(bool),
    Null,
    Undefined,
    
    // Variables and Functions
    Identifier(String),
    VariableDecl {
        name: String,
        init: Option<Box<Node>>,
    },
    FunctionDecl {
        name: String,
        params: Vec<String>,
        body: Vec<Node>,
    },
    
    // Expressions
    BinaryOp {
        op: BinaryOperator,
        left: Box<Node>,
        right: Box<Node>,
    },
    CallExpr {
        callee: Box<Node>,
        arguments: Vec<Node>,
    },
    
    // Control Flow
    IfStatement {
        condition: Box<Node>,
        consequent: Box<Node>,
        alternate: Option<Box<Node>>,
    },
    WhileLoop {
        condition: Box<Node>,
        body: Box<Node>,
    },
    
    // Program root
    Program(Vec<Node>),
}

#[derive(Debug, Clone)]
pub enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Equal,
    NotEqual,
    // Add more operators as needed
} 