use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub enum Mod {
    Resolved {
        name: String,
        author: String,
        // this is not Optional because the filename is not shown to the user,
        // so they cannot guess the version, so this should be the correct or the best guess
        version: String,
        dependency: bool,
        required: bool,
        link: String,
        icon_url: String,
    },
    Unresolved {
        filename: String,
        // this is Optional because the user can guess the version from the filename too,
        // so this should either be correct or not there
        version: Option<String>,
        author: Option<String>,
        website: Option<String>,
    },
}
