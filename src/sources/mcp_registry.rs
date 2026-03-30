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
    pub author: String,
    pub date: String,
    pub homepage: String,
    pub repository: String,
    pub npm_url: String,
    pub keywords: Vec<String>,
    pub score_quality: f64,
    pub score_popularity: f64,
    pub score_maintenance: f64,
}

impl RegistryEntry {
    /// Generate a rich preview body for display in the detail pane.
    pub fn preview_body(&self) -> String {
        let mut lines = Vec::new();

        lines.push(format!("# {}", self.name));
        lines.push(String::new());

        if !self.description.is_empty() {
            lines.push(self.description.clone());
            lines.push(String::new());
        }

        lines.push("---".to_string());
        lines.push(String::new());

        lines.push(format!("Version: {}", self.version));
        if !self.author.is_empty() {
            lines.push(format!("Author: {}", self.author));
        }
        if !self.date.is_empty() {
            lines.push(format!(
                "Published: {}",
                &self.date[..self.date.len().min(10)]
            ));
        }
        lines.push(format!("Registry: {}", self.registry));

        lines.push(String::new());

        // Score bars
        lines.push("## Quality".to_string());
        lines.push(format!("  {}", score_bar(self.score_quality)));
        lines.push("## Popularity".to_string());
        lines.push(format!("  {}", score_bar(self.score_popularity)));
        lines.push("## Maintenance".to_string());
        lines.push(format!("  {}", score_bar(self.score_maintenance)));

        lines.push(String::new());

        // Links
        let has_links =
            !self.npm_url.is_empty() || !self.homepage.is_empty() || !self.repository.is_empty();
        if has_links {
            lines.push("---".to_string());
            lines.push(String::new());
            if !self.npm_url.is_empty() {
                lines.push(format!("- npm: {}", self.npm_url));
            }
            if !self.homepage.is_empty() {
                lines.push(format!("- Homepage: {}", self.homepage));
            }
            if !self.repository.is_empty() {
                lines.push(format!("- Repository: {}", self.repository));
            }
            lines.push(String::new());
        }

        // Keywords
        if !self.keywords.is_empty() {
            lines.push("---".to_string());
            lines.push(String::new());
            lines.push(format!("Keywords: {}", self.keywords.join(", ")));
            lines.push(String::new());
        }

        // Install command
        lines.push("---".to_string());
        lines.push(String::new());
        lines.push("## Install".to_string());
        lines.push(format!(
            "  {} {}",
            self.install_command,
            self.install_args.join(" ")
        ));

        lines.join("\n")
    }

    /// Return a popularity indicator string (filled/empty circles).
    pub fn popularity_dots(&self) -> &'static str {
        let level = (self.score_popularity * 5.0).round() as u8;
        match level {
            0 => "○○○○○",
            1 => "●○○○○",
            2 => "●●○○○",
            3 => "●●●○○",
            4 => "●●●●○",
            _ => "●●●●●",
        }
    }
}

/// Render a score (0.0–1.0) as a visual bar.
fn score_bar(score: f64) -> String {
    let filled = (score * 10.0).round().clamp(0.0, 10.0) as usize;
    let empty = 10 - filled.min(10);
    format!(
        "{}{} {:.0}%",
        "█".repeat(filled),
        "░".repeat(empty),
        score * 100.0
    )
}

#[derive(Deserialize)]
struct NpmSearchResponse {
    objects: Vec<NpmSearchObject>,
}

#[derive(Deserialize)]
struct NpmSearchObject {
    package: NpmPackage,
    score: Option<NpmScore>,
}

#[derive(Deserialize)]
struct NpmScore {
    detail: Option<NpmScoreDetail>,
}

#[derive(Deserialize)]
struct NpmScoreDetail {
    quality: Option<f64>,
    popularity: Option<f64>,
    maintenance: Option<f64>,
}

#[derive(Deserialize)]
struct NpmPackage {
    name: String,
    description: Option<String>,
    version: String,
    keywords: Option<Vec<String>>,
    date: Option<String>,
    links: Option<NpmLinks>,
    author: Option<NpmAuthor>,
    publisher: Option<NpmPublisher>,
}

#[derive(Deserialize)]
struct NpmLinks {
    npm: Option<String>,
    homepage: Option<String>,
    repository: Option<String>,
}

#[derive(Deserialize)]
struct NpmAuthor {
    name: Option<String>,
}

#[derive(Deserialize)]
struct NpmPublisher {
    username: Option<String>,
}

/// Search the npm registry for MCP server packages.
/// Pass an empty query to browse popular MCP packages.
pub fn search_npm(query: &str) -> Result<Vec<RegistryEntry>, String> {
    tracing::info!("Searching npm registry for MCP servers (query={:?})", query);
    let url = if query.is_empty() {
        "https://registry.npmjs.org/-/v1/search?text=keywords:mcp&size=50".to_string()
    } else {
        format!(
            "https://registry.npmjs.org/-/v1/search?text=keywords:mcp+{}&size=50",
            urlencoded(query)
        )
    };

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
            let score_detail = obj.score.and_then(|s| s.detail);
            let author = pkg
                .author
                .and_then(|a| a.name)
                .or_else(|| pkg.publisher.and_then(|p| p.username))
                .unwrap_or_default();
            let links = pkg.links.unwrap_or(NpmLinks {
                npm: None,
                homepage: None,
                repository: None,
            });
            RegistryEntry {
                install_command: "npx".to_string(),
                install_args: vec!["-y".to_string(), pkg.name.clone()],
                npm_url: links.npm.unwrap_or_default(),
                homepage: links.homepage.unwrap_or_default(),
                repository: links.repository.unwrap_or_default(),
                author,
                date: pkg.date.unwrap_or_default(),
                keywords: pkg.keywords.unwrap_or_default(),
                score_quality: score_detail.as_ref().and_then(|d| d.quality).unwrap_or(0.0),
                score_popularity: score_detail
                    .as_ref()
                    .and_then(|d| d.popularity)
                    .unwrap_or(0.0),
                score_maintenance: score_detail
                    .as_ref()
                    .and_then(|d| d.maintenance)
                    .unwrap_or(0.0),
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
            _ => {
                let mut s = String::new();
                for byte in c.to_string().as_bytes() {
                    s.push_str(&format!("%{:02X}", byte));
                }
                s
            }
        })
        .collect()
}
