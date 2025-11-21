//! Groups impl blocks and their methods to reduce output noise.
//!
//! This module provides two types of grouping:
//! 1. Groups methods into impl blocks: impl Foo { fn bar(); fn baz(); }
//! 2. Condenses std library trait impls: impl Debug + Display + Clone for Foo

use crate::namespace_manager::NamespaceManager;
use crate::public_item::PublicItem;
use rustdoc_types::{Crate, Id, ItemEnum};
use std::collections::HashMap;

/// Represents either a single item or a group of related items
#[derive(Debug)]
pub enum ItemGroup {
    /// A single item that isn't part of a group
    Single(PublicItem),
    /// A group of std trait impl blocks for the same type (condensed)
    TraitImplGroup {
        /// All impl items in this group
        members: Vec<PublicItem>,
    },
    /// An impl block with its methods grouped together
    ImplWithMethods {
        /// The impl block itself
        impl_item: PublicItem,
        /// Methods belonging to this impl
        methods: Vec<PublicItem>,
    },
}

/// Groups impl blocks and their methods
pub fn group_impl_items(items: Vec<PublicItem>, crate_data: &Crate) -> Vec<ItemGroup> {
    // First, separate items by type
    let mut impl_blocks = Vec::new();
    let mut trait_impls: HashMap<String, Vec<(PublicItem, Id)>> = HashMap::new();
    let mut methods_by_parent: HashMap<Id, Vec<PublicItem>> = HashMap::new();
    let mut other_items = Vec::new();

    for item in items {
        let item_id = item.id();

        if let Some(rustdoc_item) = crate_data.index.get(&item_id) {
            match &rustdoc_item.inner {
                ItemEnum::Impl(impl_data) => {
                    if let Some(trait_path) = &impl_data.trait_ {
                        // This is a trait impl - check if it's a std trait
                        let trait_id = trait_path.id;
                        if is_std_trait(trait_id, crate_data) {
                            // Only group concrete (non-generic, no where clause) impls
                            // Blanket impls with generics should stay separate
                            let text = item.to_string();

                            // Check if this is a concrete impl (no generics, no where clause)
                            let is_concrete = !text.contains('<') && !text.contains(" where");

                            if is_concrete {
                                // Group concrete impls by the "for Type" part
                                let grouping_key = if let Some(for_pos) = text.find(" for ") {
                                    let after_for = &text[for_pos + 5..];
                                    after_for.trim_end_matches(';').trim().to_string()
                                } else {
                                    continue;
                                };

                                trait_impls
                                    .entry(grouping_key)
                                    .or_default()
                                    .push((item, trait_id));
                            } else {
                                // Don't group generic/blanket impls - output them individually
                                other_items.push(ItemGroup::Single(item));
                            }
                        } else {
                            other_items.push(ItemGroup::Single(item));
                        }
                    } else {
                        // Inherent impl - collect it to group with methods later
                        impl_blocks.push(item);
                    }
                }
                ItemEnum::Function(_)
                | ItemEnum::AssocConst { .. }
                | ItemEnum::AssocType { .. } => {
                    // These are methods/associated items - group by parent
                    if let Some(parent) = item.parent_id() {
                        methods_by_parent.entry(parent).or_default().push(item);
                    } else {
                        other_items.push(ItemGroup::Single(item));
                    }
                }
                _ => {
                    other_items.push(ItemGroup::Single(item));
                }
            }
        } else {
            other_items.push(ItemGroup::Single(item));
        }
    }

    let mut result = Vec::new();

    // Group std trait impls by type
    for (_for_type, mut impls) in trait_impls {
        if impls.len() > 1 {
            // Sort by trait name for consistent output
            impls.sort_by(|(a, _), (b, _)| a.to_string().cmp(&b.to_string()));

            let members = impls.into_iter().map(|(item, _)| item).collect();
            result.push(ItemGroup::TraitImplGroup { members });
        } else {
            result.push(ItemGroup::Single(impls.into_iter().next().unwrap().0));
        }
    }

    // Group impl blocks with their methods
    for impl_item in impl_blocks {
        let impl_id = impl_item.id();
        if let Some(methods) = methods_by_parent.remove(&impl_id) {
            if !methods.is_empty() {
                result.push(ItemGroup::ImplWithMethods { impl_item, methods });
                continue;
            }
        }
        result.push(ItemGroup::Single(impl_item));
    }

    // Add remaining items
    result.extend(other_items);

    result
}

/// Check if a trait (by its ID) is from the standard library
fn is_std_trait(trait_id: Id, crate_data: &Crate) -> bool {
    // Look up the trait in the crate paths
    if let Some(item_summary) = crate_data.paths.get(&trait_id) {
        // Check if the path starts with a std crate
        if let Some(first_component) = item_summary.path.first() {
            matches!(first_component.as_str(), "std" | "core" | "alloc")
        } else {
            false
        }
    } else {
        false
    }
}

/// Render a condensed group of trait impls
pub fn render_trait_impl_group(members: &[PublicItem], namespace_mgr: &NamespaceManager) -> String {
    if members.is_empty() {
        return String::new();
    }

    // Extract trait names (just the trait, not generics/where clauses)
    let mut traits = Vec::new();
    let mut for_type = None;

    for member in members {
        let text = member.to_string();
        // Parse "impl Trait for Type"
        if let Some(for_pos) = text.find(" for ") {
            let mut impl_part = &text[4..for_pos]; // Skip "impl"

            // Skip impl generics like "<T>" or "<T, U>"
            if impl_part.trim_start().starts_with('<') {
                if let Some(close_angle) = impl_part.find('>') {
                    impl_part = &impl_part[close_angle + 1..];
                }
            }

            impl_part = impl_part.trim();

            // Now extract just the trait name (before any trait generics)
            let trait_name = if let Some(angle_pos) = impl_part.find('<') {
                impl_part[..angle_pos].trim()
            } else {
                impl_part
            };

            if !trait_name.is_empty() {
                traits.push(trait_name.to_string());
            }

            if for_type.is_none() {
                let after_for = &text[for_pos + 5..];
                let type_part = if let Some(where_pos) = after_for.find(" where") {
                    &after_for[..where_pos]
                } else {
                    after_for.trim_end_matches(';')
                };
                for_type = Some(type_part.trim().to_string());
            }
        }
    }

    if let Some(for_type) = for_type {
        let result = format!("impl {} for {}", traits.join(" + "), for_type);
        namespace_mgr.shorten_text(&result)
    } else {
        // Fallback to first item
        namespace_mgr.shorten_text(&members[0].to_string())
    }
}

/// Render an impl block with its methods
pub fn render_impl_with_methods(
    impl_item: &PublicItem,
    methods: &[PublicItem],
    namespace_mgr: &NamespaceManager,
) -> String {
    let impl_line = namespace_mgr.shorten_text(&impl_item.to_string());

    if methods.is_empty() {
        return impl_line;
    }

    let mut result = impl_line.trim_end_matches(';').to_string();
    result.push_str(" {\n");

    for method in methods {
        let method_str = namespace_mgr.shorten_text(&method.to_string());
        // Methods are already rendered without type prefix thanks to prepare_items_for_grouping
        result.push_str("    ");
        result.push_str(method_str.trim_end_matches(';'));
        result.push_str(";\n");
    }

    result.push('}');
    result
}
