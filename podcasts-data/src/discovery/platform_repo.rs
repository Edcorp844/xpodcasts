use crate::discovery::FoundPodcast;
use anyhow::Result;

pub trait Repository {
    async fn search(&self, query: &str) -> Result<Vec<FoundPodcast>>;
    async fn get_top(&self, limit: i32, language: Option<&str>) -> Result<Vec<FoundPodcast>>;
}
