pub mod rss_generator;

#[derive(Default, Debug)]
pub struct Ddys {
    pub title: String,
    pub category: Vec<String>,
    pub url: String,
    pub desc: String,
    pub image_url: String,
}

