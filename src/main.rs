use anyhow::{Context, Result};
use clap::Parser;
use regex::Regex;
use std::path::PathBuf;
use std::process::Command;

mod crate_wrapper;
mod error;
mod impl_grouper;
mod intermediate_public_item;
mod item_processor;
mod nameable_item;
mod namespace_manager;
mod path_component;
mod public_item;
mod render;
mod tokens;

use namespace_manager::NamespaceManager;
use public_item::PublicItem;

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

/// Builder options (simplified - no filtering for now)
#[derive(Copy, Clone, Debug)]
#[allow(dead_code)]
struct BuilderOptions {
    sorted: bool,
    debug_sorting: bool,
    omit_blanket_impls: bool,
    omit_auto_trait_impls: bool,
    omit_auto_derived_impls: bool,
}

impl Default for BuilderOptions {
    fn default() -> Self {
        Self {
            sorted: true,
            debug_sorting: false,
            omit_blanket_impls: false,
            omit_auto_trait_impls: false,
            omit_auto_derived_impls: false,
        }
    }
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Step 1: Build rustdoc JSON
    let json_path = build_rustdoc_json(&args)?;

    // Step 2: Parse the JSON
    let json_content =
        std::fs::read_to_string(&json_path).context("Failed to read rustdoc JSON file")?;

    let crate_data: rustdoc_types::Crate =
        serde_json::from_str(&json_content).context("Failed to parse rustdoc JSON")?;

    // Step 3: Extract public items using the item processor
    let options = BuilderOptions::default();
    let public_items = public_api_from_crate(&crate_data, options)?;

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
            impl_grouper::ItemGroup::ImplGroup {
                impl_item,
                members,
                is_std_trait,
            } => {
                let rendered = impl_grouper::render_impl_group(
                    &impl_item,
                    &members,
                    is_std_trait,
                    &namespace_mgr,
                );
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

fn public_api_from_crate(
    crate_: &rustdoc_types::Crate,
    options: BuilderOptions,
) -> Result<Vec<PublicItem>> {
    let public_api = item_processor::public_api_in_crate(crate_, options);
    Ok(public_api.items)
}

/// The public API of a crate
pub struct PublicApi {
    /// The items that constitute the public API
    pub items: Vec<PublicItem>,

    /// Missing item IDs
    pub missing_item_ids: Vec<u32>,
}

fn build_rustdoc_json(args: &Args) -> Result<PathBuf> {
    let mut cmd = Command::new("rustup");
    cmd.args(["run", "nightly", "cargo", "rustdoc", "--lib"]);
    cmd.arg("--manifest-path");
    cmd.arg(&args.manifest_path);

    if let Some(package) = &args.package {
        cmd.args(["--package", package]);
    }

    cmd.args(["--", "-Z", "unstable-options", "--output-format", "json"]);

    let status = cmd.status().context("Failed to execute cargo rustdoc")?;

    if !status.success() {
        anyhow::bail!("cargo rustdoc failed");
    }

    // Determine the output path
    let package_name = if let Some(package) = &args.package {
        package.clone()
    } else {
        get_package_name(&args.manifest_path)?
    };

    let json_path = PathBuf::from("target/doc")
        .join(package_name.replace('-', "_"))
        .with_extension("json");

    if !json_path.exists() {
        anyhow::bail!(
            "Expected rustdoc JSON at {:?} but it doesn't exist",
            json_path
        );
    }

    Ok(json_path)
}

fn get_package_name(manifest_path: &PathBuf) -> Result<String> {
    let manifest_content =
        std::fs::read_to_string(manifest_path).context("Failed to read Cargo.toml")?;

    // Simple TOML parsing to extract package name
    for line in manifest_content.lines() {
        let line = line.trim();
        if line.starts_with("name") && line.contains('=') {
            let parts: Vec<&str> = line.split('=').collect();
            if parts.len() == 2 {
                let name = parts[1].trim().trim_matches('"').trim_matches('\'');
                return Ok(name.to_string());
            }
        }
    }

    anyhow::bail!("Could not find package name in Cargo.toml")
}
