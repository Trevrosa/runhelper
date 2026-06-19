use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub enum Mod {
    Resolved {
        name: String,
        required: bool,
        link: String,
        icon_url: String,
    },
    Unresolved {
        filename: String,
        author: Option<String>,
        website: Option<String>,
    },
}
