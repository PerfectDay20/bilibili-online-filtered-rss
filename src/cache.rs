use std::collections::HashMap;
use tokio::time::Instant;

pub struct RssCache {
    cache: HashMap<CacheType, Content>
}

pub struct Content {
    rss: String,
    record_instant: Instant,
}

#[derive(Debug, Eq, PartialEq, Hash)]
pub enum CacheType {
    Bilibili, Ddys
}

impl RssCache {
    pub fn new() -> Self {
        RssCache{
            cache: HashMap::new()
        }
    }

    pub fn get(&self, cache_type: &CacheType) -> Option<&Content> {
        self.cache.get(cache_type)
    }

    pub fn insert(&mut self, cache_type: CacheType, rss:String) {
        self.cache.insert(cache_type, Content::new(rss));
    }
}

impl Content {
    pub fn new(rss: String) -> Self {
        Self {
            rss,
            record_instant: Instant::now()
        }
    }

    /// Content expired after 10 minutes
    pub fn is_expired(&self) -> bool{
        self.record_instant.elapsed().as_secs() > 600
    }

    pub fn get_rss(&self) -> String {
        self.rss.clone()
    }
}
