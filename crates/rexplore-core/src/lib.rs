pub mod crate_wrapper;
pub mod error;
pub mod impl_grouper;
pub mod intermediate_public_item;
pub mod item_processor;
pub mod nameable_item;
pub mod namespace_manager;
pub mod path_component;
pub mod public_item;
pub mod render;
pub mod tokens;

pub use error::RexploreError;
pub use namespace_manager::NamespaceManager;
pub use public_item::PublicItem;

/// The public API of a crate
#[derive(Debug)]
pub struct Api {
    /// The items that constitute the public API
    pub items: Vec<PublicItem>,

    /// Missing item IDs
    pub missing_item_ids: Vec<u32>,
}

pub fn public_api_in_crate(crate_: &rustdoc_types::Crate, options: BuilderOptions) -> Api {
    item_processor::public_api_in_crate(crate_, options)
}

use rustdoc_types::Crate;
use std::path::Path;

#[derive(Copy, Clone, Debug)]
pub struct BuilderOptions {
    pub sorted: bool,
    pub debug_sorting: bool,
    pub omit_blanket_impls: bool,
    pub omit_auto_trait_impls: bool,
    pub omit_auto_derived_impls: bool,
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

pub fn build_rustdoc_json(
    manifest_path: &Path,
    package: Option<&str>,
) -> Result<std::path::PathBuf, RexploreError> {
    use std::process::Command;

    let mut cmd = Command::new("rustup");
    cmd.args(["run", "nightly", "cargo", "rustdoc", "--lib"]);
    cmd.arg("--manifest-path");
    cmd.arg(manifest_path);

    if let Some(package) = package {
        cmd.args(["--package", package]);
    }

    cmd.args(["--", "-Z", "unstable-options", "--output-format", "json"]);

    let status = cmd.status().map_err(|e| {
        RexploreError::CommandExecution(format!("Failed to execute cargo rustdoc: {}", e))
    })?;

    if !status.success() {
        return Err(RexploreError::RustdocFailed);
    }

    let package_name = if let Some(package) = package {
        package.to_string()
    } else {
        get_package_name(manifest_path)?
    };

    let json_path = std::path::PathBuf::from("target/doc")
        .join(package_name.replace('-', "_"))
        .with_extension("json");

    if !json_path.exists() {
        return Err(RexploreError::RustdocOutputNotFound(json_path));
    }

    Ok(json_path)
}

pub fn get_package_name(manifest_path: &Path) -> Result<String, RexploreError> {
    let manifest_content = std::fs::read_to_string(manifest_path).map_err(RexploreError::Io)?;

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

    Err(RexploreError::PackageNameNotFound)
}

pub fn load_rustdoc_json(path: &Path) -> Result<Crate, RexploreError> {
    let json_content = std::fs::read_to_string(path).map_err(RexploreError::Io)?;

    serde_json::from_str(&json_content).map_err(RexploreError::JsonParse)
}
