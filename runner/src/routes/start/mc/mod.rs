use std::path::Path;

pub mod forge;
pub mod paper;

#[derive(Debug)]
pub enum ServerType {
    Forge,
    Paper,
    Vanilla,
}

impl ServerType {
    pub fn detect(server_path: &Path) -> Option<Self> {
        if server_path.join("libraries/net/minecraftforge").exists() {
            Some(Self::Forge)
        } else if server_path.join("libraries/com/velocitypowered").exists() {
            Some(Self::Paper)
        } else if server_path.join("libraries/com/mojang").exists() {
            Some(Self::Vanilla)
        } else {
            None
        }
    }
}