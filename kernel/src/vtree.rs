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

pub mod vfs {
    use alloc::{
        alloc::{alloc, realloc},
        string::*,
    };
    use core::{
        alloc::{Layout, LayoutError},
        borrow::Borrow as _,
        fmt::Debug,
        mem::ManuallyDrop,
        ops::{Deref as _, DerefMut},
        ptr::{null, null_mut},
    };

    use crate::{output_n, TransmuteInto as _};

    pub type Filename = [char; 26];

    #[allow(non_upper_case_globals)]
    pub const fname: for<'a> fn(&'a str) -> Filename = self::slice_to_array::<26>;

    #[inline]
    pub fn slice_to_array<'a, const N: usize>(value: &'a str) -> [char; N] {
        debug_assert!(value.len() <= N);

        let mut array_builder: [char; N] = [' '; N];

        for (i, chr) in value.char_indices() {
            if chr.is_ascii_control() || chr == '\0' {
                continue;
            }

            array_builder[i] = chr;
        }

        array_builder
    }

    #[derive(Debug)]
    pub enum GenericError {
        Layout(LayoutError),
        AllocationFailed,
        UnexpectedNullPointer,
        InvalidNode(NodeType),
    }

    #[repr(u8)]
    #[derive(Eq, PartialEq)]
    pub enum NodeType {
        Null,
        Directory,
        File,
    }

    impl Debug for NodeType {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            f.write_str(self.descriptor())
        }
    }

    impl NodeType {
        pub fn descriptor(&self) -> &'static str {
            match self {
                Self::Null => "<null>",
                Self::Directory => "dir",
                Self::File => "file",
                _ => unimplemented!(),
            }
        }
    }

    #[derive(Debug)]
    #[repr(C)]
    pub struct Directory {
        pub nodes: u32,
        pub compacity: u32,
        pub first: *mut INode,
    }

    const SUBNODE_COMPACITY_INC: u32 = 2;
    const INITIAL_DIRECTORY_COMPACITY: u32 = 2;

    impl Directory {
        /// Creates an new [Directory] Node with a pre-defined sub-node compacity.
        ///
        /// Note: We use [alloc] directly becuase of c-compatibility.
        pub unsafe fn with_compacity(compacity: u32) -> Result<Self, GenericError> {
            let layout = Layout::array::<INode>(compacity as usize)
                .map_err(|err| GenericError::Layout(err))?;

            let ptr = alloc(layout) as *mut INode;

            if ptr.is_null() {
                Err(GenericError::AllocationFailed)
            } else {
                Ok(Self {
                    nodes: 0,
                    compacity,
                    first: ptr,
                })
            }
        }

        pub fn find(&self, name: Filename) -> Option<&INode> {
            for node in unsafe { self.node_slice() } {
                if node.name == name {
                    return Some(node);
                }
            }

            None
        }

        pub fn create_directory(&mut self, name: Filename) -> Result<&mut Directory, GenericError> {
            match self.add_node(INode::wrapped_directory(name, unsafe {
                Directory::with_compacity(INITIAL_DIRECTORY_COMPACITY)?
            })) {
                Ok(node) => node
                    .directory_mut()
                    .ok_or(GenericError::InvalidNode(NodeType::Directory)),
                Err(err) => Err(err),
            }
        }

        pub fn write_text_file(&mut self, name: Filename, text: &str) -> Result<&mut Self, GenericError>{
            let (buffer, size) = unsafe {
                let layout = Layout::for_value(text);

                let buffer = alloc(layout);

                buffer.copy_from(text.as_ptr(), layout.size());

                (buffer, layout.size())
            };

            _ = self.add_node(INode::new_file(name, buffer, unsafe { size.transmute() }))?;

            Ok(self)
        }

        pub fn find_recursive(&self, path: &str) -> Option<&INode> {

            let mut filename: Filename;
            let mut optional_rest: &str = "";

            if !path.contains('/') {
                filename = fname(path);
            } else {
                let (slice, rest) = path.split_once('/')?;
                optional_rest = rest;
                filename = fname(slice);
            };

            let node = self.find(filename)?;

            if !optional_rest.is_empty() {
                node.directory()?.find_recursive(optional_rest)
            } else {
                Some(node)
            }
        }

        /// Adds an [INode] as a sub-node to a directory.
        ///
        /// If the directory doesn't have enough compacity left, it increases the compacity.
        pub fn add_node(&mut self, node: INode) -> Result<&mut INode, GenericError> {
            output_n(
                "Filesystem",
                alloc::format!(
                    "Creating sub-node of type {} named {}...",
                    node.ty.descriptor(),
                    node.name()
                )
                .as_str(),
            );

            if self.nodes == self.compacity {
                unsafe { self._grow_compacity_internal(SUBNODE_COMPACITY_INC)? }
            }

            self.nodes += 1;

            unsafe {
                self.first.add((self.nodes - 1) as usize).write(node);

                self.first
                    .add((self.nodes - 1) as usize)
                    .as_mut()
                    .ok_or(GenericError::UnexpectedNullPointer)
            }
        }

        pub unsafe fn node_slice(&self) -> &[INode] {
            core::slice::from_raw_parts(self.first, self.nodes as usize)
        }

        pub(crate) unsafe fn _grow_compacity_internal(
            &mut self,
            amount: u32,
        ) -> Result<(), GenericError> {
            let layout = Layout::array::<INode>((self.compacity + amount) as usize)
                .map_err(|err| GenericError::Layout(err))?;

            let ptr = realloc(
                self.first as *mut u8,
                layout,
                (self.compacity + amount) as usize,
            ) as *mut INode;

            if ptr.is_null() {
                return Err(GenericError::AllocationFailed);
            };

            self.compacity += amount;
            self.first = ptr;

            Ok(())
        }
    }

    #[derive(Debug)]
    #[repr(C)]
    pub struct File {
        pub size: u64,
        pub base: *const u8,
    }

    #[repr(C)]
    pub struct INode {
        pub name: Filename,
        pub ty: NodeType,
        pub data: Node,
    }

    impl INode {
        pub fn name(&self) -> String {
            String::from_iter(self.name.iter())
        }

        pub fn directory(&self) -> Option<&Directory> {
            if self.ty == NodeType::Directory {
                Some(unsafe { self.data.directory.deref() })
            } else {
                None
            }
        }

        pub fn file(&self) -> Option<&File> {
            if self.ty == NodeType::File {
                Some(unsafe { self.data.file.deref() })
            } else {
                None
            }
        }

        pub fn directory_mut(&mut self) -> Option<&mut Directory> {
            if self.ty == NodeType::Directory {
                Some(unsafe { self.data.directory.deref_mut() })
            } else {
                None
            }
        }

        pub fn file_mut(&mut self) -> Option<&mut File> {
            if self.ty == NodeType::File {
                Some(unsafe { self.data.file.deref_mut() })
            } else {
                None
            }
        }

        pub fn new_file(name: Filename, base: *const u8, size: u64) -> Self {
            Self {
                name,
                ty: NodeType::File,
                data: Node {
                    file: ManuallyDrop::new(File { base, size }),
                },
            }
        }

        pub fn wrapped_directory(name: Filename, dir: Directory) -> Self {
            Self {
                name,
                ty: NodeType::Directory,
                data: Node {
                    directory: ManuallyDrop::new(dir),
                },
            }
        }
    }

    impl Debug for INode {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            f.debug_tuple(self.name().as_str())
                .field(match self.ty {
                    NodeType::Null => &"<null>",
                    NodeType::Directory => unsafe { &self.data.directory },
                    NodeType::File => unsafe { &self.data.file },
                    _ => unimplemented!(),
                })
                .finish()
        }
    }

    #[repr(C)]
    pub union Node {
        directory: ManuallyDrop<Directory>,
        file: ManuallyDrop<File>,
    }

    #[derive(Debug)]
    #[repr(C)]
    pub struct VirtualFilesystem {
        root_dir: Directory,
    }

    // If we don't do this we can't make it global...!
    unsafe impl Send for VirtualFilesystem {}
    unsafe impl Sync for VirtualFilesystem {}

    impl VirtualFilesystem {
        pub fn new() -> Self {
            Self {
                root_dir: unsafe {
                    Directory::with_compacity(INITIAL_DIRECTORY_COMPACITY).unwrap()
                },
            }
        }

        pub fn root(&self) -> &Directory {
            &self.root_dir
        }

        pub fn root_mut(&mut self) -> &mut Directory {
            &mut self.root_dir
        }
    }
}
