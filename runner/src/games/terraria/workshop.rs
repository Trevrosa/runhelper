use std::{env, sync::LazyLock};

use anyhow::anyhow;
use reqwest::Client;
use serde::Deserialize;

static API_KEY: LazyLock<String> =
    LazyLock::new(|| env::var("STEAM_APIKEY").expect("should be set"));

#[derive(Debug, Deserialize)]
struct Response {
    response: SearchResponse,
}

#[derive(Debug, Deserialize)]
struct SearchResponse {
    publishedfiledetails: Vec<SearchItem>,
}

#[derive(Debug, Deserialize)]
pub struct SearchItem {
    #[serde(rename = "publishedfileid")]
    file_id: String,
    pub preview_url: String,
    kvtags: Vec<ItemTag>,
}

impl SearchItem {
    pub fn url(&self) -> String {
        format!(
            "https://steamcommunity.com/sharedfiles/filedetails/?id={}",
            self.file_id
        )
    }

    pub fn name(&self) -> &str {
        self.tag_value("name").expect("required")
    }

    pub fn author(&self) -> &str {
        self.tag_value("Author").expect("required")
    }

    pub fn modside(&self) -> &str {
        self.tag_value("modside").expect("required")
    }

    fn tag_value(&self, key: &str) -> Option<&str> {
        self.kvtags
            .iter()
            .find(|t| t.key == key)
            .map(|t| t.value.as_ref())
    }
}

#[derive(Debug, Deserialize)]
pub struct ItemTag {
    pub key: String,
    pub value: String,
}

/// returns the best result
pub async fn search(client: &Client, name: &str) -> anyhow::Result<SearchItem> {
    const URL: &str = "https://api.steampowered.com/IPublishedFileService/QueryFiles/v1/";

    let query = &[
        ("key", API_KEY.as_str()),
        ("query_type", "0"), // https://partner.steamgames.com/doc/webapi/IPublishedFileService#EPublishedFileQueryType
        ("page", "1"),
        ("numperpage", "3"),
        ("appid", "1281930"),         // tmodloader
        ("creator_appid", "1281930"), // tmodloader
        ("search_text", name),
        ("filetype", "0"), // https://partner.steamgames.com/doc/webapi/IPublishedFileService#EPublishedFileInfoMatchingFileType
        ("return_metadata", "true"),
        ("return_kv_tags", "true"),
    ];

    let resp: Response = client.get(URL).query(query).send().await?.json().await?;

    let result = resp
        .response
        .publishedfiledetails
        .into_iter()
        .find(|i| i.name() == name)
        .ok_or(anyhow!("no matching result"))?;

    Ok(result)
}
