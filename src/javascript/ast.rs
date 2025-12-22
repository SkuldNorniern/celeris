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
    FunctionExpr {
        name: Option<String>,  // Optional for anonymous functions
        params: Vec<String>,
        body: Vec<Node>,
    },
    ArrowFunction {
        params: Vec<String>,
        body: Box<Node>,  // Can be expression or block
    },
    
    // Expressions
    BinaryOp {
        op: BinaryOperator,
        left: Box<Node>,
        right: Box<Node>,
    },
    UnaryOp {
        op: UnaryOperator,
        operand: Box<Node>,
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
    ForLoop {
        init: Option<Box<Node>>,
        condition: Option<Box<Node>>,
        update: Option<Box<Node>>,
        body: Box<Node>,
    },
    ForInLoop {
        variable: String,
        object: Box<Node>,
        body: Box<Node>,
    },
    ForOfLoop {
        variable: String,
        iterable: Box<Node>,
        body: Box<Node>,
    },
    Block(Vec<Node>),
    ReturnStatement(Option<Box<Node>>),
    BreakStatement,
    ContinueStatement,
    ThrowStatement(Box<Node>),
    TryCatch {
        try_block: Box<Node>,
        catch_param: Option<String>,
        catch_block: Option<Box<Node>>,
        finally_block: Option<Box<Node>>,
    },
    This,
    
    // Program root
    Program(Vec<Node>),
    
    // Add new expression types
    MemberExpr {
        object: Box<Node>,
        property: Box<Node>,
        computed: bool,  // true for obj[prop], false for obj.prop
    },
    LogicalOr {
        left: Box<Node>,
        right: Box<Node>,
    },
    
    // Add assignment expression
    AssignmentExpr {
        left: Box<Node>,
        right: Box<Node>,
    },
    
    // Add array literal
    ArrayLiteral(Vec<Node>),
    
    // Add new expression
    NewExpr {
        constructor: Box<Node>,
        arguments: Vec<Node>,
    },
    
    // Add object literal
    ObjectLiteral(Vec<(String, Box<Node>)>),
    
    // Comma operator: evaluates all expressions, returns last
    CommaExpr(Vec<Node>),
    
    // Ternary operator: condition ? true_expr : false_expr
    TernaryExpr {
        condition: Box<Node>,
        true_expr: Box<Node>,
        false_expr: Box<Node>,
    },
}

#[derive(Debug, Clone)]
pub enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Equal,          // ==
    StrictEqual,    // ===
    NotEqual,       // !=
    StrictNotEqual, // !==
    LessThan,
    GreaterThan,
    LessThanEqual,
    GreaterThanEqual,
    LogicalAnd,     // &&
    LogicalOr,      // ||
    Instanceof,     // instanceof
    In,             // in
}

#[derive(Debug, Clone)]
pub enum UnaryOperator {
    Not,           // !
    Typeof,
    Negative,      // -
    PostIncrement, // ++ (postfix)
    PostDecrement, // -- (postfix)
    PreIncrement,  // ++ (prefix)
    PreDecrement,  // -- (prefix)
} 