use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct FrontmatterInfo {
    pub title: String,
    pub date: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub tags: Vec<String>,
}