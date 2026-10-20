use crate::discovery::FoundPodcast;
use anyhow::Result;

pub trait Repository {
    fn search(
        &self,
        query: &str,
    ) -> impl std::future::Future<Output = Result<Vec<FoundPodcast>>> + Send;
    fn get_top(
        &self,
        limit: i32,
        language: Option<&str>,
    ) -> impl std::future::Future<Output = Result<Vec<FoundPodcast>>> + Send;
}
