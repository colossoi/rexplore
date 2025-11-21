//! Groups impl blocks for standard library traits to reduce output noise.
//!
//! Instead of showing separate impl blocks like:
//!   impl Debug for Foo;
//!   impl Display for Foo;
//!   impl Clone for Foo;
//!
//! This module groups them as:
//!   impl Debug + Display + Clone for Foo;

use crate::namespace_manager::NamespaceManager;
use crate::public_item::PublicItem;
use crate::tokens::{tokens_to_string, Token};
use rustdoc_types::Crate;
use std::collections::HashMap;

/// Represents either a single item or a group of related impl blocks
#[derive(Debug)]
pub enum ItemGroup {
    /// A single item that isn't part of a group
    Single(PublicItem),
    /// A group of impl blocks for the same type
    ImplGroup {
        /// The first impl item (used as a template for rendering)
        impl_item: PublicItem,
        /// All impl items in this group (including the first)
        members: Vec<PublicItem>,
        /// Whether all traits in this group are from std library
        is_std_trait: bool,
    },
}

/// Information extracted from an impl block
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ImplInfo {
    /// The trait being implemented (if any), e.g., "Debug" or "std::fmt::Display"
    trait_name: Option<String>,
    /// The type being implemented for, e.g., "Foo" or "Vec<T>"
    for_type: String,
    /// Whether this impl is for a std library trait
    is_std_trait: bool,
}

/// Groups impl blocks that implement standard library traits for the same type
pub fn group_impl_items(items: Vec<PublicItem>, _crate_data: &Crate) -> Vec<ItemGroup> {
    let mut impl_groups: HashMap<String, Vec<(PublicItem, ImplInfo)>> = HashMap::new();
    let mut other_items = Vec::new();

    // Separate impl blocks from other items
    for item in items {
        if let Some(impl_info) = parse_impl_block(&item) {
            // Only group trait impls (not inherent impls)
            if impl_info.trait_name.is_some() {
                impl_groups
                    .entry(impl_info.for_type.clone())
                    .or_insert_with(Vec::new)
                    .push((item, impl_info));
            } else {
                other_items.push(ItemGroup::Single(item));
            }
        } else {
            other_items.push(ItemGroup::Single(item));
        }
    }

    // Group impls by type and check if they can be condensed
    let mut result = Vec::new();
    for (_for_type, mut impls) in impl_groups {
        // Check if all impls are for std traits
        let all_std_traits = impls.iter().all(|(_, info)| info.is_std_trait);

        if all_std_traits && impls.len() > 1 {
            // Sort by trait name for consistent output
            impls.sort_by(|(_, a), (_, b)| a.trait_name.cmp(&b.trait_name));

            let members: Vec<PublicItem> = impls.iter().map(|(item, _)| item.clone()).collect();
            let impl_item = impls[0].0.clone();

            result.push(ItemGroup::ImplGroup {
                impl_item,
                members,
                is_std_trait: true,
            });
        } else {
            // Don't group - add each impl separately
            for (item, _) in impls {
                result.push(ItemGroup::Single(item));
            }
        }
    }

    // Add back non-impl items
    result.extend(other_items);

    result
}

/// Parse an impl block to extract trait and type information
fn parse_impl_block(item: &PublicItem) -> Option<ImplInfo> {
    let tokens = &item.tokens;
    let text = tokens_to_string(tokens);

    // Check if this is an impl block
    if !text.contains("impl") {
        return None;
    }

    // Find the "impl" keyword position
    let mut impl_pos = None;
    for (i, token) in tokens.iter().enumerate() {
        if matches!(token, Token::Keyword(k) if k == "impl") {
            impl_pos = Some(i);
            break;
        }
    }

    let impl_pos = impl_pos?;

    // Look for "for" keyword to determine if this is a trait impl
    let mut for_pos = None;
    for (i, token) in tokens.iter().enumerate().skip(impl_pos) {
        if matches!(token, Token::Keyword(k) if k == "for") {
            for_pos = Some(i);
            break;
        }
    }

    if let Some(for_pos) = for_pos {
        // This is a trait impl: "impl Trait for Type"
        let trait_tokens = &tokens[impl_pos + 1..for_pos];
        let trait_name = extract_trait_name(trait_tokens);
        let is_std_trait = is_std_library_trait(&trait_name);

        let for_type = extract_for_type(&tokens[for_pos + 1..]);

        Some(ImplInfo {
            trait_name: Some(trait_name),
            for_type,
            is_std_trait,
        })
    } else {
        // This is an inherent impl: "impl Type"
        let for_type = extract_for_type(&tokens[impl_pos + 1..]);

        Some(ImplInfo {
            trait_name: None,
            for_type,
            is_std_trait: false,
        })
    }
}

/// Extract the trait name from tokens between "impl" and "for"
fn extract_trait_name(tokens: &[Token]) -> String {
    let mut result = String::new();
    for token in tokens {
        match token {
            Token::Whitespace => {}
            Token::Keyword(k) if k == "where" => break,
            Token::Symbol(s) if s == "<" => break, // Stop at generic params
            _ => result.push_str(token.text()),
        }
    }
    result.trim().to_string()
}

/// Extract the type name from tokens after "for" or after "impl" (for inherent impls)
fn extract_for_type(tokens: &[Token]) -> String {
    let mut result = String::new();
    let mut depth: i32 = 0; // Track generic depth

    for token in tokens {
        match token {
            Token::Keyword(k) if k == "where" && depth == 0 => break,
            Token::Symbol(s) if s == "<" => {
                result.push_str(s);
                depth += 1;
            }
            Token::Symbol(s) if s == ">" => {
                result.push_str(s);
                depth = depth.saturating_sub(1);
            }
            Token::Whitespace => {
                if depth > 0 {
                    result.push(' ');
                }
            }
            _ => result.push_str(token.text()),
        }
    }

    result.trim().to_string()
}

/// Check if a trait is from the standard library
fn is_std_library_trait(trait_name: &str) -> bool {
    trait_name.starts_with("std::")
        || trait_name.starts_with("core::")
        || trait_name.starts_with("alloc::")
}

/// Render a group of impl blocks as a single condensed line
pub fn render_impl_group(
    impl_item: &PublicItem,
    members: &[PublicItem],
    is_std_trait: bool,
    namespace_mgr: &NamespaceManager,
) -> String {
    if !is_std_trait || members.len() <= 1 {
        // Fall back to single rendering
        return namespace_mgr.shorten_text(&impl_item.to_string());
    }

    // Extract trait names from all members
    let mut trait_names = Vec::new();
    for member in members {
        if let Some(impl_info) = parse_impl_block(member) {
            if let Some(trait_name) = impl_info.trait_name {
                trait_names.push(trait_name);
            }
        }
    }

    // Extract the "for Type" part from the first impl
    let first_impl_info = parse_impl_block(impl_item).expect("impl_item should be parseable");

    // Build the condensed impl string
    let traits = trait_names.join(" + ");
    let result = format!("impl {} for {}", traits, first_impl_info.for_type);

    // Apply namespace shortening
    namespace_mgr.shorten_text(&result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_std_library_trait() {
        assert!(is_std_library_trait("std::fmt::Debug"));
        assert!(is_std_library_trait("core::marker::Send"));
        assert!(is_std_library_trait("alloc::string::ToString"));
        assert!(!is_std_library_trait("MyTrait"));
        assert!(!is_std_library_trait("my_crate::MyTrait"));
    }
}
