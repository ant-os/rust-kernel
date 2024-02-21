use core::fmt::{Debug, Display};

use alloc::{borrow::ToOwned, collections::BTreeMap as HashMap, string::*, vec::*};

pub enum Node {
    Directory(HashMap<String, Node>),
    Link(String),
}

impl Debug for Node {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Directory(_) => f.write_str("{NAME}"),
            Self::Link(target) => f.write_fmt(format_args!("{{NAME}} -> {}", target)),
            _ => unimplemented!(),
        }
    }
}

pub struct VTree {
    pub(crate) nodes: HashMap<String, Node>,
}

impl VTree {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
        }
    }

    pub fn find(&self, path: &str) -> Result<&Node, &str> {
        // @ PREFIX SUBTREE SEGMENTS
        // # //./   .       .../...

        let mut path = path.to_string();

        if !path.starts_with("//") {
            return Err("invalid path prefix.");
        }

        path = path.trim_start_matches("//").to_owned();

        let segments = path.split("/").collect::<Vec<_>>();

        if segments.is_empty() {
            return Err("no segments in path.");
        }

        let mut iterator = segments.iter();

        let mut current_node = self
            .nodes
            .get(
                &iterator
                    .next()
                    .expect("internal: subtree of path schould never be none")
                    .to_string(),
            )
            .ok_or("subtree not found")?;

        for segment in iterator {
            if segment.is_empty() {
                continue;
            }

            match &current_node {
                Node::Directory(ref subtree) => {
                    current_node = subtree.get(&(*segment).to_string()).ok_or("not found")?
                }
                _ => todo!(),
            }
        }

        Ok(current_node)
    }

    pub fn builder(&mut self, global: &'static str) -> Option<TreeBuilder> {
        let node = self.nodes.get_mut(global)?;

        Some(TreeBuilder(node))
    }
}

impl Node {
    pub fn empty_directory() -> Self {
        Self::Directory(HashMap::new())
    }

    pub fn children(&self) -> Vec<(String, &Node)> {
        let mut tmp = Vec::<(String, &Node)>::new();

        tmp.push(("<self>".to_owned(), self));

        tmp
    }
}

pub struct TreeBuilder<'a>(&'a mut Node);

impl<'a> TreeBuilder<'a> {
    pub fn attach_or_update(&mut self, name: String, node: Node) -> &mut Self {
        if let Node::Directory(ref mut subtree) = self.0 {
            subtree.insert(name, node);
        }

        self
    }
}
