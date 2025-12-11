use std::path::Path;

pub mod tmodloader;

#[derive(Debug)]
pub enum ServerType {
    Vanilla,
    TModLoader,
}

impl ServerType {
    pub fn detect(server_path: &Path) -> Option<Self> {
        if server_path.join("TerrariaServer.exe").exists() {
            Some(Self::Vanilla)
        } else if server_path.join("tModLoader.dll").exists() {
            Some(Self::TModLoader)
        } else {
            None
        }
    }
}