//! Manages namespace prefixes and generates `use` statements to simplify output.
//!
//! This module analyzes all public items to determine which namespace prefixes
//! can be safely elided, generates appropriate `use` statements, and provides
//! a way to render items with shortened paths.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

/// Tracks namespace usage and determines which can be safely shortened
pub struct NamespaceManager {
    /// Maps simple names to all their fully qualified paths
    /// e.g., "Send" -> ["core::marker::Send"]
    pub name_to_paths: HashMap<String, HashSet<String>>,

    /// Namespaces that can be safely elided (no name collisions)
    /// Maps "core::marker::Send" -> "Send"
    pub safe_shortenings: HashMap<String, String>,

    /// Maps namespace prefix to all the items from that namespace
    /// e.g., "core::marker" -> ["Send", "Sync", "Unpin"]
    namespace_to_items: BTreeMap<String, BTreeSet<String>>,
}

impl NamespaceManager {
    pub fn new() -> Self {
        Self {
            name_to_paths: HashMap::new(),
            safe_shortenings: HashMap::new(),
            namespace_to_items: BTreeMap::new(),
        }
    }

    /// Analyze a collection of strings (output lines) to find all type references
    pub fn analyze(&mut self, items: &[String]) {
        for item in items {
            self.extract_and_record_paths(item);
        }
        self.compute_safe_shortenings();
    }

    /// Extract all qualified paths from a string and record them
    fn extract_and_record_paths(&mut self, text: &str) {
        // Look for patterns like "core::marker::Send", "std::vec::Vec", etc.
        // We need to be careful to extract complete paths
        let mut chars = text.chars().peekable();
        let mut current_path = String::new();
        let mut in_path = false;

        while let Some(ch) = chars.next() {
            if ch.is_alphanumeric() || ch == '_' {
                current_path.push(ch);
                in_path = true;
            } else if ch == ':' && chars.peek() == Some(&':') {
                // Double colon
                current_path.push_str("::");
                chars.next(); // consume second ':'
                in_path = true;
            } else if in_path {
                // End of path
                if current_path.contains("::") {
                    self.record_path(&current_path);
                }
                current_path.clear();
                in_path = false;
            }
        }

        // Check final path
        if in_path && current_path.contains("::") {
            self.record_path(&current_path);
        }
    }

    /// Record a single qualified path
    fn record_path(&mut self, full_path: &str) {
        if let Some((namespace, name)) = extract_namespace_and_name(full_path) {
            // Only record if the name looks like a type/trait (uppercase start)
            // This helps avoid recording function names, parameters, etc.
            if let Some(first_char) = name.chars().next() {
                if first_char.is_uppercase() || name == "never" {
                    // Count the namespace depth (number of ::)
                    let depth = namespace.matches("::").count();

                    // Process all qualified paths:
                    // - For single-level (e.g., "anyhow::Error"), we still record to remove the prefix
                    // - For multi-level (e.g., "core::marker::Send", "anyhow::context::Error"),
                    //   we record normally to generate use statements

                    // Record this path
                    self.name_to_paths
                        .entry(name.to_string())
                        .or_insert_with(HashSet::new)
                        .insert(full_path.to_string());

                    // Only add to namespace_to_items if depth > 0 (multi-level namespace)
                    // This way single-level prefixes like "anyhow::" will be removed but won't
                    // generate a use statement
                    if depth > 0 {
                        self.namespace_to_items
                            .entry(namespace.to_string())
                            .or_insert_with(BTreeSet::new)
                            .insert(name.to_string());
                    }
                }
            }
        }
    }

    /// Compute which paths can be safely shortened (no name collisions)
    fn compute_safe_shortenings(&mut self) {
        for (simple_name, paths) in &self.name_to_paths {
            // Only shorten if there's no ambiguity
            if paths.len() == 1 {
                let full_path = paths.iter().next().unwrap();
                self.safe_shortenings
                    .insert(full_path.clone(), simple_name.clone());
            }
        }
    }

    /// Generate `use` statements grouped by namespace
    pub fn generate_use_statements(&self) -> Vec<String> {
        let mut statements = Vec::new();

        // Group items by namespace prefix that can be shortened
        let mut prefix_groups: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

        for (full_path, short_name) in &self.safe_shortenings {
            if let Some((namespace, _)) = extract_namespace_and_name(full_path) {
                prefix_groups
                    .entry(namespace.to_string())
                    .or_insert_with(BTreeSet::new)
                    .insert(short_name.clone());
            }
        }

        // Generate use statements
        for (namespace, items) in prefix_groups {
            // Skip empty namespaces (these are associated types or similar)
            if namespace.is_empty() {
                continue;
            }

            if items.len() == 1 {
                // Single item: use std::vec::Vec;
                let item = items.iter().next().unwrap();
                statements.push(format!("use {}::{};", namespace, item));
            } else {
                // Multiple items: use std::ops::{Range, RangeFrom};
                let items_list: Vec<_> = items.iter().cloned().collect();
                statements.push(format!("use {}::{{{}}};", namespace, items_list.join(", ")));
            }
        }

        statements
    }

    /// Shorten a text by replacing full paths with their short forms
    pub fn shorten_text(&self, text: &str) -> String {
        let mut result = text.to_string();

        // Sort by length (longest first) to avoid partial replacements
        let mut shortenings: Vec<_> = self.safe_shortenings.iter().collect();
        shortenings.sort_by_key(|(full, _)| std::cmp::Reverse(full.len()));

        for (full_path, short_name) in shortenings {
            result = result.replace(full_path, short_name);
        }

        result
    }

    /// Get statistics about namespace usage
    #[allow(dead_code)]
    pub fn stats(&self) -> NamespaceStats {
        NamespaceStats {
            total_unique_names: self.name_to_paths.len(),
            names_with_collisions: self
                .name_to_paths
                .iter()
                .filter(|(_, paths)| paths.len() > 1)
                .count(),
            shortenable_paths: self.safe_shortenings.len(),
            unique_namespaces: self.namespace_to_items.len(),
        }
    }
}

/// Statistics about namespace usage
#[derive(Debug)]
#[allow(dead_code)]
pub struct NamespaceStats {
    pub total_unique_names: usize,
    pub names_with_collisions: usize,
    pub shortenable_paths: usize,
    pub unique_namespaces: usize,
}

/// Extract namespace and final name from a qualified path
/// e.g., "core::marker::Send" -> Some(("core::marker", "Send"))
fn extract_namespace_and_name(path: &str) -> Option<(&str, &str)> {
    path.rfind("::").map(|pos| {
        let namespace = &path[..pos];
        let name = &path[pos + 2..];
        (namespace, name)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_namespace_and_name() {
        assert_eq!(
            extract_namespace_and_name("core::marker::Send"),
            Some(("core::marker", "Send"))
        );
        assert_eq!(
            extract_namespace_and_name("std::vec::Vec"),
            Some(("std::vec", "Vec"))
        );
        assert_eq!(extract_namespace_and_name("simple"), None);
    }

    #[test]
    fn test_no_collisions() {
        let mut mgr = NamespaceManager::new();
        let items = vec![
            "impl core::marker::Send for Foo".to_string(),
            "impl core::marker::Sync for Foo".to_string(),
        ];

        mgr.analyze(&items);

        let use_stmts = mgr.generate_use_statements();
        assert!(use_stmts
            .iter()
            .any(|s| s.contains("core::marker::{Send, Sync}")));

        let shortened = mgr.shorten_text("impl core::marker::Send for Foo");
        assert_eq!(shortened, "impl Send for Foo");
    }

    #[test]
    fn test_with_collisions() {
        let mut mgr = NamespaceManager::new();
        let items = vec![
            "type anyhow::Result = core::result::Result".to_string(),
            "fn foo() -> std::io::Result".to_string(),
        ];

        mgr.analyze(&items);

        // Result appears in multiple namespaces, should not be shortened
        let shortened = mgr.shorten_text("type anyhow::Result = core::result::Result");
        assert!(shortened.contains("core::result::Result"));
        assert!(shortened.contains("anyhow::Result"));
    }
}
