use anyhow::anyhow;
use reqwest::Client;
use serde::Deserialize;

use crate::routes::start::Mod;

#[derive(Debug, Deserialize)]
pub struct Project {
    pub title: String,
    pub slug: String,
    pub icon_url: String,
    // client side support (required, optional, unsupported, etc)
    pub client_side: String,
    pub author: String,
}

#[derive(Deserialize)]
struct SearchResult {
    hits: Vec<Project>,
}

impl Project {
    pub async fn find(client: &Client, query: &str) -> anyhow::Result<Self> {
        const API: &str = "https://api.modrinth.com/v2/search";
        let query: String = query.chars().filter(|c| !c.is_numeric()).collect();
        let resp = client.get(API).query(&[("query", &query)]).send().await?;
        let mut result = resp.json::<SearchResult>().await?.hits;
        if result.is_empty() {
            return Err(anyhow!("no search results"));
        }
        Ok(result.swap_remove(0))
    }
}

impl From<Project> for Mod {
    fn from(val: Project) -> Self {
        Mod::Resolved {
            name: val.title,
            author: val.author,
            required: val.client_side == "required",
            link: format!("https://modrinth.com/mod/{}", val.slug),
            icon_url: val.icon_url,
        }
    }
}
