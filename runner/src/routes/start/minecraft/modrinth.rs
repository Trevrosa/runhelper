use anyhow::Context;
use reqwest::Client;
use serde::Deserialize;

use super::Mod;

#[derive(Debug, Deserialize)]
pub struct Project {
    pub title: String,
    pub slug: String,
    pub icon_url: String,
    /// client side support (required, optional, unsupported, etc)
    pub client_side: String,
    pub author: String,
    pub categories: Vec<String>,
}

#[derive(Deserialize)]
struct SearchResult {
    hits: Vec<Project>,
}

impl Project {
    pub async fn find(client: &Client, query: &str) -> anyhow::Result<Vec<Self>> {
        const API: &str = "https://api.modrinth.com/v2/search";
        let query: String = query.chars().filter(|c| !c.is_numeric()).collect();
        let resp = client.get(API).query(&[("query", &query)]).send().await;
        let resp = resp.context("sending search req")?;
        Ok(resp.json::<SearchResult>().await?.hits)
    }
}

impl From<Project> for Mod {
    fn from(val: Project) -> Self {
        Mod::Resolved {
            name: val.title,
            author: val.author,
            dependency: val.categories.iter().any(|s| s == "library"),
            required: val.client_side == "required",
            link: format!("https://modrinth.com/mod/{}", val.slug),
            icon_url: val.icon_url,
        }
    }
}
