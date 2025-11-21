use thiserror::Error;

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum RexploreError {
    #[error("Failed to parse rustdoc JSON: {0}")]
    JsonParse(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Failed to execute command: {0}")]
    CommandExecution(String),

    #[error("cargo rustdoc command failed. Ensure Rust nightly toolchain is installed: rustup toolchain install nightly")]
    RustdocFailed,

    #[error("Rustdoc JSON output not found at: {0}")]
    RustdocOutputNotFound(std::path::PathBuf),

    #[error("Could not find package name in Cargo.toml")]
    PackageNameNotFound,
}

pub type Result<T> = std::result::Result<T, RexploreError>;
