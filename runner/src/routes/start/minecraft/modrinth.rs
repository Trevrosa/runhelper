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

impl From<(Project, String)> for Mod {
    fn from(val: (Project, String)) -> Self {
        let (proj, version) = val;
        Mod::Resolved {
            name: proj.title,
            author: proj.author,
            version,
            dependency: proj.categories.iter().any(|s| s == "library"),
            required: proj.client_side == "required",
            link: format!("https://modrinth.com/mod/{}", proj.slug),
            icon_url: proj.icon_url,
        }
    }
}

impl From<(Project, &str)> for Mod {
    fn from(val: (Project, &str)) -> Self {
        let val = (val.0, val.1.to_string());
        From::from(val)
    }
}
