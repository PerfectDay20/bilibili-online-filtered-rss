use std::sync::Arc;

use rss::validation::Validate;
use rss::{ChannelBuilder, GuidBuilder, ImageBuilder, Item, ItemBuilder};
use scraper::{Html, Selector};
use tokio::sync::RwLock;
use tracing::info;
use warp::{Rejection, Reply};

use crate::cache::{CacheType, RssCache};
use crate::ddys::Ddys;
use crate::error::MyError;

pub async fn generate_rss(cache: Arc<RwLock<RssCache>>) -> Result<impl Reply, Rejection> {
    if let Some(content) = cache.read().await.get(&CacheType::Ddys) {
        if !content.is_expired() {
            info!("Cache is not expired, return cache content");
            return Ok(reply(content.get_rss()));
        }
    }

    info!("Cache is None or expired, call API to generate rss");
    match generate_new_rss().await {
        Ok(rss) => {
            cache.write().await.insert(CacheType::Ddys, rss.clone());
            Ok(reply(rss))
        }
        Err(e) => Err(e),
    }
}

fn reply(rss: String) -> impl Reply {
    warp::reply::with_header(rss, "content-type", "text/xml; charset=utf-8")
}

async fn generate_new_rss() -> Result<String, Rejection> {
    let html = reqwest::get("https://ddys.pro")
        .await
        .map_err(MyError::Reqwest)?
        .text()
        .await
        .map_err(MyError::Reqwest)?;

    let fragment = Html::parse_document(&html);
    let post_selector = Selector::parse(r#" body > div[id="container"] > main > div[class="post-box-list"] > article > div[class="post-box-container"] "#).unwrap();
    let text_selector = Selector::parse(r#" div[class="post-box-text"] "#).unwrap();
    let category_selector = Selector::parse(r#" span[class="post-box-meta"] > a "#).unwrap();
    let title_selector = Selector::parse(r#" h2[class="post-box-title"] "#).unwrap();
    let desc_selector = Selector::parse("p").unwrap();
    let url_selector = Selector::parse(r#" a "#).unwrap();
    let image_selector = Selector::parse(r#" div[class="post-box-image"] "#).unwrap();

    let mut result = Vec::new();
    for post in fragment.select(&post_selector) {
        let mut ddys = Ddys::default();

        if let Some(p) = post.select(&text_selector).next() {
            // category
            for category in p.select(&category_selector) {
                for c in category.text() {
                    ddys.category.push(c.to_string());
                }
            }

            if let Some(titles) = p.select(&title_selector).next() {
                // url
                ddys.url = titles
                    .select(&url_selector)
                    .next()
                    .unwrap()
                    .value()
                    .attr("href")
                    .unwrap()
                    .to_string();
                if let Some(title) = titles.text().next() {
                    // title
                    ddys.title = title.to_string();
                }
            }
            // desc
            if let Some(desc) = p.select(&desc_selector).next() {
                ddys.desc = desc.text().collect::<Vec<_>>().join(" ").to_string();
            }
        }

        // image
        if let Some(image) = post.select(&image_selector).next() {
            let style = image.value().attr("style").unwrap();
            let left_index = style.find('(').unwrap_or(0);
            let right_index = style.rfind(')').unwrap_or(style.len());
            ddys.image_url = style[left_index + 1..right_index].to_string();
        }

        result.push(ddys);
    }
    assemble(result)
}

const TITLE: &str = "ddys.site";
const LINK: &str = "https://ddys.site";
const DESC: &str = "A rss for ddys";
const ICON_URL: &str = "https://ddys.art/favicon-32x32.png";

fn assemble(items: Vec<Ddys>) -> Result<String, Rejection> {
    let channel = ChannelBuilder::default()
        .title(TITLE)
        .link(LINK)
        .description(DESC)
        .image(Some(
            ImageBuilder::default()
                .title(TITLE)
                .link(LINK)
                .url(ICON_URL)
                .build(),
        ))
        .items(
            items
                .iter()
                .map(|d| {
                    ItemBuilder::default()
                        .title(d.title.clone())
                        .description(create_item_desc(d))
                        .link(d.url.clone())
                        .guid(
                            // guid = title + url, because when an episode is updated, the page title is changed while url is not,
                            // we don't know which one will be used by rss aggregator to deduplicate,
                            // so make the guid explicit is a safe bet
                            GuidBuilder::default()
                                .value(d.title.clone() + &d.url)
                                .permalink(false)
                                .build(),
                        )
                        .build()
                })
                .collect::<Vec<Item>>(),
        )
        .build();

    channel.validate().map_err(MyError::Validation)?;
    Ok(channel.to_string())
}

fn create_item_desc(d: &Ddys) -> String {
    format!(
        r#"
    <b>category:</b> {category}
    <p></p>
    <b>desc:</b> {desc}
    <p></p>
    <img style="width:100%" src="{img_src}" width="500">"#,
        category = d.category.join(" "),
        desc = d.desc,
        img_src = d.image_url
    )
}
