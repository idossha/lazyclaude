//! MCP server discovery via package registries.
//!
//! Searches npm for packages tagged with `mcp` keyword.
//! Extensible to other registries (Smithery, PyPI, etc.) in the future.

use serde::Deserialize;

/// A discovered MCP server package from a registry.
#[derive(Clone, Debug, serde::Serialize)]
pub struct RegistryEntry {
    pub name: String,
    pub description: String,
    pub version: String,
    pub install_command: String,
    pub install_args: Vec<String>,
    pub registry: String,
}

#[derive(Deserialize)]
struct NpmSearchResponse {
    objects: Vec<NpmSearchObject>,
}

#[derive(Deserialize)]
struct NpmSearchObject {
    package: NpmPackage,
}

#[derive(Deserialize)]
struct NpmPackage {
    name: String,
    description: Option<String>,
    version: String,
}

/// Search the npm registry for MCP server packages.
pub fn search_npm(query: &str) -> Result<Vec<RegistryEntry>, String> {
    let url = format!(
        "https://registry.npmjs.org/-/v1/search?text=keywords:mcp+{}&size=25",
        urlencoded(query)
    );

    let response = ureq::get(&url)
        .call()
        .map_err(|e| format!("HTTP error: {e}"))?;

    let result: NpmSearchResponse = response
        .into_json()
        .map_err(|e| format!("JSON parse error: {e}"))?;

    Ok(result
        .objects
        .into_iter()
        .map(|obj| {
            let pkg = obj.package;
            RegistryEntry {
                install_command: "npx".to_string(),
                install_args: vec!["-y".to_string(), pkg.name.clone()],
                name: pkg.name,
                description: pkg.description.unwrap_or_default(),
                version: pkg.version,
                registry: "npm".to_string(),
            }
        })
        .collect())
}

/// Simple percent-encoding for URL query params.
fn urlencoded(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
            ' ' => "+".to_string(),
            _ => format!("%{:02X}", c as u8),
        })
        .collect()
}
