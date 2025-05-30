use std::fs;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use tui_tree_widget::TreeItem;

/// Represents a node in the directory tree
#[derive(Debug, Clone)]
pub struct TreeNode {
    pub name: String,
    pub path: PathBuf,
    pub is_file: bool,
    pub children: Vec<TreeNode>,
}

pub struct ProjectService;

impl ProjectService {
    pub fn new() -> Self {
        Self
    }

    /// Generates a tree control from a recursive directory of *.md files
    pub fn generate_md_tree<P: AsRef<Path>>(
        &self,
        root_path: P,
    ) -> Result<(Vec<TreeItem<'static, String>>, HashMap<String,u32>), Box<dyn std::error::Error>> {
        let root = root_path.as_ref();

        if !root.exists() {
            return Err(format!("Path does not exist: {}", root.display()).into());
        }

        if !root.is_dir() {
            return Err(format!("Path is not a directory: {}", root.display()).into());
        }

        let tree_nodes = self.scan_directory(root)?;
        let (tree_items, tree_id_map) = self.convert_nodes_to_tree_items(tree_nodes)?;

        Ok((tree_items, tree_id_map))
    }

    /// Recursively scans a directory and builds tree nodes
    fn scan_directory(&self, dir_path: &Path) -> Result<Vec<TreeNode>, Box<dyn std::error::Error>> {
        let mut nodes = Vec::new();
        let mut dirs = Vec::new();
        let mut files = Vec::new();

        // Read directory entries
        let entries = fs::read_dir(dir_path)?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();

            if path.is_dir() {
                // Skip hidden directories
                if name.starts_with('.') {
                    continue;
                }

                let children = self.scan_directory(&path)?;

                // Only include directories that contain .md files (directly or in subdirectories)
                if self.has_md_files(&children) {
                    dirs.push(TreeNode {
                        name,
                        path,
                        is_file: false,
                        children,
                    });
                }
            } else if path.is_file() {
                // Only include .md files
                if let Some(extension) = path.extension() {
                    if extension == "md" {
                        files.push(TreeNode {
                            name,
                            path,
                            is_file: true,
                            children: Vec::new(),
                        });
                    }
                }
            }
        }

        // Sort directories and files alphabetically
        dirs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        files.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

        // Add directories first, then files
        nodes.extend(dirs);
        nodes.extend(files);

        Ok(nodes)
    }

    /// Checks if a tree node or its children contain any .md files
    fn has_md_files(&self, nodes: &[TreeNode]) -> bool {
        nodes.iter().any(|node| {
            if node.is_file {
                true
            } else {
                self.has_md_files(&node.children)
            }
        })
    }

    /// Converts TreeNode structure to TreeItem structure for the TUI widget
    fn convert_nodes_to_tree_items(
        &self,
        nodes: Vec<TreeNode>,
    ) -> Result<(Vec<TreeItem<'static, String>>, HashMap<String,u32>), Box<dyn std::error::Error>> {
        let mut tree_items = Vec::new();
        let mut id_counter = 0u32;
        let mut id_map = HashMap::new();

        for node in nodes {
            let tree_item = self.convert_node_to_tree_item(&node, &mut id_counter, &mut id_map)?;
            tree_items.push(tree_item);
        }

        Ok((tree_items, id_map))
    }

    /// Recursively converts a single TreeNode to TreeItem
    fn convert_node_to_tree_item(
        &self,
        node: &TreeNode,
        id_counter: &mut u32,
        id_map: &mut HashMap<String, u32>,
    ) -> Result<TreeItem<'static, String>, Box<dyn std::error::Error>> {
        let id = self.generate_unique_id(&node.path, id_counter, id_map);
        let display_name = if node.is_file {
            // Remove .md extension for display
            node.name.strip_suffix(".md").unwrap_or(&node.name).to_string()
        } else {
            format!("{}", node.name)
        };

        if node.children.is_empty() {
            // Leaf node (file)
            Ok(TreeItem::new_leaf(id, format!("{}", display_name)))
        } else {
            // Branch node (directory with children)
            let mut child_items = Vec::new();

            for child in &node.children {
                let child_item = self.convert_node_to_tree_item(child, id_counter, id_map)?;
                child_items.push(child_item);
            }

            TreeItem::new(id, display_name, child_items)
                .map_err(|e| format!("Failed to create tree item: {}", e).into())
        }
    }

    /// Generates a unique ID for each tree item
    fn generate_unique_id(
        &self,
        path: &Path,
        id_counter: &mut u32,
        id_map: &mut HashMap<String, u32>,
    ) -> String {
        let path_str = path.to_string_lossy().to_string();

        if let Some(&existing_id) = id_map.get(&path_str) {
            return existing_id.to_string();
        }

        let id = *id_counter;
        *id_counter += 1;
        id_map.insert(path_str, id);

        id.to_string()
    }

    /// Alternative function that returns TreeNode structure if you need more control
    pub fn scan_md_directory<P: AsRef<Path>>(
        &self,
        root_path: P,
    ) -> Result<Vec<TreeNode>, Box<dyn std::error::Error>> {
        let root = root_path.as_ref();

        if !root.exists() {
            return Err(format!("Path does not exist: {}", root.display()).into());
        }

        if !root.is_dir() {
            return Err(format!("Path is not a directory: {}", root.display()).into());
        }

        self.scan_directory(root)
    }
}