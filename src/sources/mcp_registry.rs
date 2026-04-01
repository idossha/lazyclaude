//! MCP server discovery via package registries.
//!
//! Searches multiple sources for MCP server packages:
//! - npm registry (packages tagged with `mcp` keyword)
//! - Official MCP Registry (registry.modelcontextprotocol.io)
//! - Smithery (registry.smithery.ai)

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

        if !self.version.is_empty() {
            lines.push(format!("Version: {}", self.version));
        }
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

        // Score bars (only if scores are non-zero, i.e. from npm)
        if self.score_quality > 0.0 || self.score_popularity > 0.0 || self.score_maintenance > 0.0 {
            lines.push("## Quality".to_string());
            lines.push(format!("  {}", score_bar(self.score_quality)));
            lines.push("## Popularity".to_string());
            lines.push(format!("  {}", score_bar(self.score_popularity)));
            lines.push("## Maintenance".to_string());
            lines.push(format!("  {}", score_bar(self.score_maintenance)));
            lines.push(String::new());
        }

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

// ── npm registry ───────────────────────────────────────────────────────

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
        .map_err(|e| format!("npm: HTTP error: {e}"))?;

    let result: NpmSearchResponse = response
        .into_json()
        .map_err(|e| format!("npm: JSON parse error: {e}"))?;

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

// ── Official MCP Registry ──────────────────────────────────────────────

#[derive(Deserialize)]
struct OfficialResponse {
    servers: Option<Vec<OfficialServerWrapper>>,
}

#[derive(Deserialize)]
struct OfficialServerWrapper {
    server: OfficialServer,
}

#[derive(Deserialize)]
struct OfficialServer {
    name: Option<String>,
    description: Option<String>,
    version: Option<String>,
    #[serde(rename = "websiteUrl")]
    website_url: Option<String>,
    repository: Option<OfficialRepository>,
    packages: Option<Vec<OfficialPackage>>,
}

#[derive(Deserialize)]
struct OfficialRepository {
    url: Option<String>,
}

#[derive(Deserialize)]
struct OfficialPackage {
    #[serde(rename = "registryType")]
    registry_type: Option<String>,
    identifier: Option<String>,
}

/// Search the official MCP Registry at registry.modelcontextprotocol.io.
pub fn search_official(query: &str) -> Result<Vec<RegistryEntry>, String> {
    tracing::info!("Searching official MCP registry (query={:?})", query);

    let url = if query.is_empty() {
        "https://registry.modelcontextprotocol.io/v0.1/servers?limit=100".to_string()
    } else {
        format!(
            "https://registry.modelcontextprotocol.io/v0.1/servers?limit=100&search={}",
            urlencoded(query)
        )
    };

    let response = ureq::get(&url)
        .set("User-Agent", "lazyclaude")
        .call()
        .map_err(|e| format!("MCP Registry: HTTP error: {e}"))?;

    let result: OfficialResponse = response
        .into_json()
        .map_err(|e| format!("MCP Registry: JSON parse error: {e}"))?;

    let servers = result.servers.unwrap_or_default();

    Ok(servers
        .into_iter()
        .filter_map(|wrapper| {
            let srv = wrapper.server;
            let raw_name = srv.name?;

            // Pick the best installable package (prefer npm, then pypi)
            let pkg = srv.packages.as_ref().and_then(|pkgs| {
                pkgs.iter()
                    .find(|p| p.registry_type.as_deref() == Some("npm"))
                    .or_else(|| {
                        pkgs.iter()
                            .find(|p| p.registry_type.as_deref() == Some("pypi"))
                    })
            });

            let (install_command, install_args, pkg_name) = match pkg {
                Some(p) => {
                    let ident = p.identifier.clone().unwrap_or_default();
                    match p.registry_type.as_deref() {
                        Some("npm") => (
                            "npx".to_string(),
                            vec!["-y".to_string(), ident.clone()],
                            ident,
                        ),
                        Some("pypi") => ("uvx".to_string(), vec![ident.clone()], ident),
                        _ => return None,
                    }
                }
                None => return None, // skip servers with no installable package
            };

            // Use package identifier as the mcp server name (clean key for .mcp.json)
            let display_name = if !pkg_name.is_empty() {
                pkg_name
            } else {
                // Fallback: extract last segment from reverse-DNS name
                raw_name.rsplit('/').next().unwrap_or(&raw_name).to_string()
            };

            let repo_url = srv.repository.and_then(|r| r.url).unwrap_or_default();

            Some(RegistryEntry {
                name: display_name,
                description: srv.description.unwrap_or_default(),
                version: srv.version.unwrap_or_default(),
                install_command,
                install_args,
                registry: "mcp-registry".to_string(),
                author: String::new(),
                date: String::new(),
                homepage: srv.website_url.unwrap_or_default(),
                repository: repo_url,
                npm_url: String::new(),
                keywords: Vec::new(),
                score_quality: 0.0,
                score_popularity: 0.0,
                score_maintenance: 0.0,
            })
        })
        .collect())
}

// ── Smithery ───────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct SmitheryResponse {
    servers: Option<Vec<SmitheryServer>>,
}

#[derive(Deserialize)]
struct SmitheryServer {
    #[serde(rename = "qualifiedName")]
    qualified_name: Option<String>,
    #[serde(rename = "displayName")]
    display_name: Option<String>,
    description: Option<String>,
    #[serde(rename = "useCount")]
    use_count: Option<u64>,
    #[serde(rename = "createdAt")]
    created_at: Option<String>,
}

/// Search the Smithery registry at registry.smithery.ai.
pub fn search_smithery(query: &str) -> Result<Vec<RegistryEntry>, String> {
    tracing::info!("Searching Smithery registry (query={:?})", query);

    let url = if query.is_empty() {
        "https://registry.smithery.ai/servers?pageSize=50".to_string()
    } else {
        format!(
            "https://registry.smithery.ai/servers?pageSize=50&q={}",
            urlencoded(query)
        )
    };

    let response = ureq::get(&url)
        .set("User-Agent", "lazyclaude")
        .call()
        .map_err(|e| format!("Smithery: HTTP error: {e}"))?;

    let result: SmitheryResponse = response
        .into_json()
        .map_err(|e| format!("Smithery: JSON parse error: {e}"))?;

    let servers = result.servers.unwrap_or_default();

    Ok(servers
        .into_iter()
        .filter_map(|srv| {
            let qualified = srv.qualified_name?;
            let display = srv
                .display_name
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| {
                    // Use slug portion of qualified name
                    qualified
                        .rsplit('/')
                        .next()
                        .unwrap_or(&qualified)
                        .to_string()
                });

            // Derive popularity score from use_count (log scale, capped at 1.0)
            let use_count = srv.use_count.unwrap_or(0) as f64;
            let popularity = if use_count > 0.0 {
                (use_count.ln() / 10_000_f64.ln()).clamp(0.0, 1.0)
            } else {
                0.0
            };

            let homepage = format!("https://smithery.ai/server/{qualified}");

            Some(RegistryEntry {
                name: display,
                description: srv.description.unwrap_or_default(),
                version: String::new(),
                install_command: "npx".to_string(),
                install_args: vec![
                    "-y".to_string(),
                    "@smithery/cli@latest".to_string(),
                    "run".to_string(),
                    qualified,
                ],
                registry: "smithery".to_string(),
                author: String::new(),
                date: srv.created_at.unwrap_or_default(),
                homepage,
                repository: String::new(),
                npm_url: String::new(),
                keywords: Vec::new(),
                score_quality: 0.0,
                score_popularity: popularity,
                score_maintenance: 0.0,
            })
        })
        .collect())
}

// ── Combined search ────────────────────────────────────────────────────

/// Search all MCP registries in parallel and merge results.
/// Deduplicates by name, preferring official registry > npm > smithery.
pub fn search_all(query: &str) -> Result<Vec<RegistryEntry>, String> {
    let q1 = query.to_string();
    let q2 = query.to_string();
    let q3 = query.to_string();

    // Fetch from all three sources in parallel
    let h_official = std::thread::spawn(move || search_official(&q1));
    let h_npm = std::thread::spawn(move || search_npm(&q2));
    let h_smithery = std::thread::spawn(move || search_smithery(&q3));

    let mut all = Vec::new();
    let mut errors = Vec::new();

    // Official registry results first (highest priority for dedup)
    match h_official.join() {
        Ok(Ok(entries)) => {
            tracing::info!("Official MCP registry: {} results", entries.len());
            all.extend(entries);
        }
        Ok(Err(e)) => {
            tracing::warn!("Official MCP registry failed: {}", e);
            errors.push(e);
        }
        Err(_) => errors.push("MCP Registry thread panicked".to_string()),
    }

    // npm results
    match h_npm.join() {
        Ok(Ok(entries)) => {
            tracing::info!("npm registry: {} results", entries.len());
            all.extend(entries);
        }
        Ok(Err(e)) => {
            tracing::warn!("npm registry failed: {}", e);
            errors.push(e);
        }
        Err(_) => errors.push("npm thread panicked".to_string()),
    }

    // Smithery results
    match h_smithery.join() {
        Ok(Ok(entries)) => {
            tracing::info!("Smithery registry: {} results", entries.len());
            all.extend(entries);
        }
        Ok(Err(e)) => {
            tracing::warn!("Smithery registry failed: {}", e);
            errors.push(e);
        }
        Err(_) => errors.push("Smithery thread panicked".to_string()),
    }

    // If ALL sources failed, return error
    if all.is_empty() && !errors.is_empty() {
        return Err(errors.join("; "));
    }

    // Deduplicate: keep first occurrence (official > npm > smithery)
    let mut seen = std::collections::HashSet::new();
    all.retain(|entry| {
        let key = entry.name.to_lowercase();
        seen.insert(key)
    });

    // Sort by popularity (descending), then name
    all.sort_by(|a, b| {
        b.score_popularity
            .partial_cmp(&a.score_popularity)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.name.cmp(&b.name))
    });

    Ok(all)
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
