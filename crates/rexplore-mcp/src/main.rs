use anyhow::{Context, Result};
use rexplore_core::{
    build_rustdoc_json, impl_grouper, load_rustdoc_json, public_api_in_crate, BuilderOptions,
    NamespaceManager,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    #[allow(dead_code)]
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    params: Option<Value>,
}

#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let stdin = tokio::io::stdin();
    let mut stdout = tokio::io::stdout();
    let mut reader = BufReader::new(stdin);
    let mut line = String::new();

    loop {
        line.clear();
        let n = reader
            .read_line(&mut line)
            .await
            .context("Failed to read from stdin")?;

        if n == 0 {
            break; // EOF
        }

        let request: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(req) => req,
            Err(e) => {
                eprintln!("Failed to parse request: {}", e);
                continue;
            }
        };

        let response = handle_request(request).await;
        let response_json = serde_json::to_string(&response)?;

        stdout
            .write_all(response_json.as_bytes())
            .await
            .context("Failed to write response")?;
        stdout
            .write_all(b"\n")
            .await
            .context("Failed to write newline")?;
        stdout.flush().await.context("Failed to flush stdout")?;
    }

    Ok(())
}

async fn handle_request(request: JsonRpcRequest) -> JsonRpcResponse {
    match request.method.as_str() {
        "initialize" => handle_initialize(request.id),
        "tools/list" => handle_tools_list(request.id),
        "tools/call" => handle_tools_call(request.id, request.params).await,
        _ => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result: None,
            error: Some(JsonRpcError {
                code: -32601,
                message: format!("Method not found: {}", request.method),
            }),
        },
    }
}

fn handle_initialize(id: Option<Value>) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result: Some(json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {}
            },
            "serverInfo": {
                "name": "rexplore-mcp",
                "version": env!("CARGO_PKG_VERSION"),
                "description": "LOCAL ONLY: Rust API explorer using rustdoc JSON. Requires filesystem access and Rust nightly compiler."
            }
        })),
        error: None,
    }
}

fn handle_tools_list(id: Option<Value>) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result: Some(json!({
            "tools": [
                {
                    "name": "explore_crate",
                    "description": "Explore the public API of a Rust crate using rustdoc JSON. Returns formatted listings of public items (modules, types, functions, traits, etc.) with optional filtering and impl grouping.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "manifest_path": {
                                "type": "string",
                                "description": "Path to Cargo.toml file. Defaults to './Cargo.toml' if not provided."
                            },
                            "package": {
                                "type": "string",
                                "description": "Package name to explore (for workspaces). If not specified, uses the default package."
                            },
                            "filter": {
                                "type": "string",
                                "description": "Optional regex pattern to filter items by their string representation."
                            },
                            "group_impls": {
                                "type": "boolean",
                                "description": "Whether to group impl blocks and condense trait impls. Default: true.",
                                "default": true
                            }
                        },
                        "required": []
                    }
                }
            ]
        })),
        error: None,
    }
}

async fn handle_tools_call(id: Option<Value>, params: Option<Value>) -> JsonRpcResponse {
    let params = match params {
        Some(p) => p,
        None => {
            return JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: None,
                error: Some(JsonRpcError {
                    code: -32602,
                    message: "Missing params".to_string(),
                }),
            }
        }
    };

    let tool_name = match params.get("name").and_then(|v| v.as_str()) {
        Some(name) => name,
        None => {
            return JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: None,
                error: Some(JsonRpcError {
                    code: -32602,
                    message: "Missing tool name".to_string(),
                }),
            }
        }
    };

    if tool_name != "explore_crate" {
        return JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code: -32602,
                message: format!("Unknown tool: {}", tool_name),
            }),
        };
    }

    let arguments = match params.get("arguments") {
        Some(args) => args,
        None => {
            return JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: None,
                error: Some(JsonRpcError {
                    code: -32602,
                    message: "Missing arguments".to_string(),
                }),
            }
        }
    };

    match execute_explore_crate(arguments).await {
        Ok(result) => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(json!({
                "content": [
                    {
                        "type": "text",
                        "text": result
                    }
                ]
            })),
            error: None,
        },
        Err(e) => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code: -32603,
                message: format!("Exploration failed: {}", e),
            }),
        },
    }
}

async fn execute_explore_crate(args: &Value) -> Result<String> {
    let manifest_path = args
        .get("manifest_path")
        .and_then(|v| v.as_str())
        .unwrap_or("./Cargo.toml");

    let package = args.get("package").and_then(|v| v.as_str());

    let filter = args
        .get("filter")
        .and_then(|v| v.as_str())
        .map(|s| regex::Regex::new(s))
        .transpose()
        .context("Invalid filter regex")?;

    let group_impls = args
        .get("group_impls")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    // Build rustdoc JSON
    let json_path = build_rustdoc_json(std::path::Path::new(manifest_path), package)
        .context("Failed to build rustdoc JSON")?;

    // Load and parse
    let crate_data = load_rustdoc_json(&json_path).context("Failed to load rustdoc JSON")?;

    // Extract public API
    let options = BuilderOptions::default();
    let public_api = public_api_in_crate(&crate_data, options);
    let public_items = public_api.items;

    // Convert to strings for filtering
    let item_strings: Vec<String> = public_items.iter().map(|item| item.to_string()).collect();

    // Apply filter if provided
    let filtered_strings = if let Some(filter_regex) = &filter {
        item_strings
            .iter()
            .filter(|s| filter_regex.is_match(s))
            .cloned()
            .collect()
    } else {
        item_strings.clone()
    };

    // Keep only the PublicItems that passed the filter
    let filtered_items = if filter.is_some() {
        public_items
            .into_iter()
            .zip(item_strings.iter())
            .filter(|(_, s)| filtered_strings.contains(s))
            .map(|(item, _)| item)
            .collect()
    } else {
        public_items
    };

    // Format output
    let mut output = String::new();

    if filtered_items.is_empty() {
        output.push_str("No public items found");
        if filter.is_some() {
            output.push_str(" (filter may be too restrictive)");
        }
        output.push('\n');
    } else {
        // Analyze namespaces
        let mut namespace_mgr = NamespaceManager::new();
        namespace_mgr.analyze(&filtered_strings);

        if group_impls {
            // Group impl blocks
            let grouped = impl_grouper::group_impl_items(filtered_items, &crate_data);

            for group in grouped {
                match group {
                    impl_grouper::ItemGroup::Single(item) => {
                        output.push_str(&namespace_mgr.shorten_text(&item.to_string()));
                        output.push('\n');
                    }
                    impl_grouper::ItemGroup::ImplWithMethods { impl_item, methods } => {
                        output.push_str(&impl_grouper::render_impl_with_methods(
                            &impl_item,
                            &methods,
                            &namespace_mgr,
                        ));
                        output.push('\n');
                    }
                    impl_grouper::ItemGroup::TraitImplGroup { members } => {
                        output.push_str(&impl_grouper::render_trait_impl_group(
                            &members,
                            &namespace_mgr,
                        ));
                        output.push('\n');
                    }
                }
            }
        } else {
            // No grouping, just list items
            for s in &filtered_strings {
                output.push_str(&namespace_mgr.shorten_text(s));
                output.push('\n');
            }
        }
    }

    Ok(output)
}
