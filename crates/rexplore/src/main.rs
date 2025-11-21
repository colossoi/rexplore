use anyhow::{Context, Result};
use clap::Parser;
use regex::Regex;
use rexplore_core::{
    build_rustdoc_json, impl_grouper, load_rustdoc_json, public_api_in_crate, BuilderOptions,
    NamespaceManager, PublicItem,
};
use std::path::PathBuf;

const LONG_ABOUT: &str = r#"Given a package name to explore, rexplore will look in the dependencies
(and sub-dependencies) of the current project to find a matching crate name.
It will then print out the complete public API as exposed by that crate,
subject to filtering with the --regex and --keyword arguments.  The API
will represent the exact crate version as used by your project.
"#;

#[derive(Parser, Debug)]
#[command(author, version, about = "Explore Rust public APIs", long_about = LONG_ABOUT)]
struct Args {
    /// Path to Cargo.toml
    #[arg(long, default_value = "Cargo.toml")]
    manifest_path: PathBuf,

    /// Package name to analyze
    #[arg(short, long)]
    package: Option<String>,

    /// Filter items by keyword substring (mutually exclusive with --regex)
    #[arg(short, long, conflicts_with = "regex")]
    keyword: Option<String>,

    /// Filter items by regex pattern (mutually exclusive with --keyword)
    #[arg(short, long, conflicts_with = "keyword")]
    regex: Option<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Step 1: Build rustdoc JSON
    let json_path = build_rustdoc_json(&args.manifest_path, args.package.as_deref())
        .context("Failed to build rustdoc JSON")?;

    // Step 2: Load and parse the JSON
    let crate_data = load_rustdoc_json(&json_path).context("Failed to load rustdoc JSON")?;

    // Step 3: Extract public items using the item processor
    let options = BuilderOptions::default();
    let public_api = public_api_in_crate(&crate_data, options);
    let public_items = public_api.items;

    // Step 4: Filter items BEFORE namespace removal
    let item_strings: Vec<String> = public_items.iter().map(|item| item.to_string()).collect();
    let filtered_strings = filter_items(&item_strings, &args)?;

    // Keep only the PublicItems that passed the filter
    let filtered_items: Vec<PublicItem> = public_items
        .into_iter()
        .zip(item_strings.iter())
        .filter(|(_, s)| filtered_strings.contains(s))
        .map(|(item, _)| item)
        .collect();

    // Step 5: Analyze namespaces and generate use statements
    let mut namespace_mgr = NamespaceManager::new();
    namespace_mgr.analyze(&filtered_strings);

    // Print use statements
    let use_statements = namespace_mgr.generate_use_statements();
    if !use_statements.is_empty() {
        for use_stmt in &use_statements {
            println!("{}", use_stmt);
        }
        println!(); // Blank line after use statements
    }

    // Step 6: Group impl items and render
    let grouped_items = impl_grouper::group_impl_items(filtered_items, &crate_data);

    for group in grouped_items {
        match group {
            impl_grouper::ItemGroup::Single(item) => {
                let shortened = namespace_mgr.shorten_text(&item.to_string());
                println!("{};", shortened);
            }
            impl_grouper::ItemGroup::TraitImplGroup { members } => {
                let rendered = impl_grouper::render_trait_impl_group(&members, &namespace_mgr);
                println!("{};", rendered);
            }
            impl_grouper::ItemGroup::ImplWithMethods { impl_item, methods } => {
                let rendered =
                    impl_grouper::render_impl_with_methods(&impl_item, &methods, &namespace_mgr);
                println!("{};", rendered);
            }
        }
    }

    Ok(())
}

fn filter_items(items: &[String], args: &Args) -> Result<Vec<String>> {
    if let Some(keyword) = &args.keyword {
        Ok(items
            .iter()
            .filter(|item| item.contains(keyword))
            .cloned()
            .collect())
    } else if let Some(pattern) = &args.regex {
        let regex = Regex::new(pattern).context(format!("Invalid regex pattern: {}", pattern))?;
        Ok(items
            .iter()
            .filter(|item| regex.is_match(item))
            .cloned()
            .collect())
    } else {
        // No filtering - return all items
        Ok(items.to_vec())
    }
}
