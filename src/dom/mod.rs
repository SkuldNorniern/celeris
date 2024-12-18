#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    node_type: NodeType,
    children: Vec<Node>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum NodeType {
    Element {
        tag_name: String,
        attributes: Vec<Attribute>,
    },
    Text(String),
    Comment(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Attribute {
    pub name: String,
    pub value: String,
}

pub struct DomTree {
    root: Option<Node>,
}

impl Node {
    pub fn new(node_type: NodeType) -> Self {
        Self {
            node_type,
            children: Vec::new(),
        }
    }

    pub fn add_child(&mut self, child: Node) {
        self.children.push(child);
    }

    pub fn node_type(&self) -> &NodeType {
        &self.node_type
    }

    pub fn children(&self) -> &[Node] {
        &self.children
    }
}

impl DomTree {
    pub fn new() -> Self {
        Self { root: None }
    }

    pub fn set_root(&mut self, node: Node) {
        self.root = Some(node);
    }

    pub fn root(&self) -> Option<&Node> {
        self.root.as_ref()
    }
}
