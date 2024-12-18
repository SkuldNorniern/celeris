use log::{debug, info, warn};

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
        debug!(target: "dom", "Creating new node: {:?}", node_type);
        Self {
            node_type,
            children: Vec::new(),
        }
    }

    pub fn add_child(&mut self, child: Node) {
        match &self.node_type {
            NodeType::Element { tag_name, .. } => {
                debug!(target: "dom", "Adding child to <{}>: {:?}", tag_name, child.node_type);
                debug!(target: "dom", "Current children count: {}", self.children.len());
            }
            NodeType::Text(text) => {
                let preview = &text[..text.len().min(20)];
                debug!(target: "dom", "Adding child to text node '{}': {:?}", preview, child.node_type);
            }
            NodeType::Comment(comment) => {
                let preview = &comment[..comment.len().min(20)];
                debug!(target: "dom", "Adding child to comment '{}': {:?}", preview, child.node_type);
            }
        }
        self.children.push(child);
        debug!(target: "dom", "New children count: {}", self.children.len());
    }

    pub fn node_type(&self) -> &NodeType {
        &self.node_type
    }

    pub fn children(&self) -> &[Node] {
        &self.children
    }

    pub fn get_attribute(&self, name: &str) -> Option<&str> {
        match &self.node_type {
            NodeType::Element { attributes, .. } => {
                let attr = attributes.iter()
                    .find(|attr| attr.name == name)
                    .map(|attr| attr.value.as_str());
                debug!(target: "dom", "Getting attribute '{}': {:?}", name, attr);
                attr
            }
            _ => None
        }
    }

    pub fn is_element(&self, tag_name: &str) -> bool {
        match &self.node_type {
            NodeType::Element { tag_name: t, .. } => {
                let is_match = t == tag_name;
                debug!(target: "dom", "Checking if node is <{}>: {}", tag_name, is_match);
                is_match
            }
            _ => false
        }
    }

    pub fn debug_print(&self, depth: usize) {
        let indent = "  ".repeat(depth);
        match &self.node_type {
            NodeType::Element { tag_name, attributes } => {
                let attrs: Vec<String> = attributes.iter()
                    .map(|attr| format!("{}=\"{}\"", attr.name, attr.value))
                    .collect();
                let attrs_str = if attrs.is_empty() {
                    String::new()
                } else {
                    format!(" {}", attrs.join(" "))
                };
                debug!(target: "dom", "{}Element: <{}{}>", indent, tag_name, attrs_str);
                debug!(target: "dom", "{}Children count: {}", indent, self.children.len());
            }
            NodeType::Text(text) => {
                if !text.trim().is_empty() {
                    debug!(target: "dom", "{}Text: {}", indent, text.trim());
                }
            }
            NodeType::Comment(comment) => {
                debug!(target: "dom", "{}Comment: {}", indent, comment);
            }
        }

        for child in &self.children {
            child.debug_print(depth + 1);
        }

        if let NodeType::Element { tag_name, .. } = &self.node_type {
            debug!(target: "dom", "{}</{}> (end)", indent, tag_name);
        }
    }
}

impl DomTree {
    pub fn new() -> Self {
        info!(target: "dom", "Creating new DOM tree");
        Self { root: None }
    }

    pub fn set_root(&mut self, node: Node) {
        info!(target: "dom", "Setting root node: {:?}", node.node_type);
        debug!(target: "dom", "Root node children count: {}", node.children.len());
        self.root = Some(node);
    }

    pub fn root(&self) -> Option<&Node> {
        if self.root.is_none() {
            warn!(target: "dom", "Attempted to access root node, but it's None");
        }
        self.root.as_ref()
    }

    pub fn root_mut(&mut self) -> Option<&mut Node> {
        if self.root.is_none() {
            warn!(target: "dom", "Attempted to access mutable root node, but it's None");
        }
        self.root.as_mut()
    }

    pub fn debug_print(&self) {
        info!(target: "dom", "=== DOM Tree Structure ===");
        if let Some(root) = &self.root {
            root.debug_print(0);
        } else {
            warn!(target: "dom", "Empty tree (no root node)");
        }
        info!(target: "dom", "=== End DOM Tree ===");
    }
}
