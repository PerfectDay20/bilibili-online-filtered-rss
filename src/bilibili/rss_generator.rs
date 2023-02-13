use std::sync::Arc;

use rss::{ChannelBuilder, ImageBuilder, Item, ItemBuilder};
use rss::validation::Validate;
use tokio::sync::RwLock;
use warp::{Rejection, Reply};

use crate::bilibili::{Bili, BiliData};
use crate::blacklist::Blacklist;
use crate::error::MyError;

const TITLE: &str = "Filtered BiliBili online list";
const LINK: &str = "https://www.bilibili.com/video/online.html";
const DESC: &str = "A filtered BiliBili online list based on my blacklist";
const ICON_URL: &str = "https://www.bilibili.com/favicon.ico";

fn create_rss(items: Vec<BiliData>) -> Result<impl Reply, Rejection> {
    let channel = ChannelBuilder::default()
        .title(TITLE)
        .link(LINK)
        .description(DESC)
        .image(Some(ImageBuilder::default()
            .title(TITLE)
            .link(LINK)
            .url(ICON_URL)
            .build()))
        .items(
            items.iter().map(|d| {
                ItemBuilder::default()
                    .title(d.title.to_string())
                    .description(create_item_desc(d))
                    .link(d.short_link.to_string())
                    .build()
            }).collect::<Vec<Item>>()
        )
        .build();

    channel.validate().map_err(MyError::Validation)?;
    Ok(warp::reply::with_header(channel.to_string(), "content-type", "text/xml; charset=utf-8"))
}

fn create_item_desc(d: &BiliData) -> String {
    format!(r#"<b>author:</b> {author}
    <p></p>
    <b>category:</b> {category}
    <p></p>
    <b>desc:</b> {desc}
    <p></p>
    <b>view:</b> {view}
    <p></p>
    <b>danmaku:</b> {danmaku}
    <p></p>
    <img style="width:100%" src="{img_src}" width="500">"#,
            author = d.owner.name,
            category = d.tname,
            desc = d.desc,
            view = convert_count(d.stat.view),
            danmaku = convert_count(d.stat.danmaku),
            img_src = d.pic)
}

/// Convert number like view count to a easier reading format,
/// for example 1000 -> 1k, 20000 -> 2w
fn convert_count(c: u32) -> String {
    if c < 1000 { c.to_string() } else if c < 10000 {
        (c / 1000).to_string() + "k"
    } else {
        (c / 10000).to_string() + "w"
    }
}

pub async fn generate_rss(blacklist: Arc<RwLock<Blacklist>>) -> Result<impl Reply, Rejection> {
    let resp = reqwest::get("https://api.bilibili.com/x/web-interface/online/list")
        .await.map_err(MyError::Reqwest)?
        .json::<Bili>()
        .await.map_err(MyError::Reqwest)?;

    let b = blacklist.read().await;
    let items: Vec<BiliData> = resp.data
        .into_iter()
        .filter(move |bili_data| b.filter(bili_data))
        .collect();

    create_rss(items)
}

#[test]
fn get_rss() {
    let channel = ChannelBuilder::default()
        .title(TITLE)
        .link(LINK)
        .description(DESC)
        .image(Some(ImageBuilder::default()
            .title(TITLE)
            .link(LINK)
            .url(ICON_URL)
            .build()))
        .items(vec![
            ItemBuilder::default()
                .title(Some("baidu".to_string()))
                .link("https://www.baidu.com".to_string())
                .description("desc1 desc1".to_string())
                .build(),
            ItemBuilder::default()
                .title(Some("bing1".to_string()))
                .link(Some("https://www.bing.com".to_string()))
                .description("desc22".to_string())
                .build(),
        ])
        .build();

    channel.validate().unwrap();
    println!("{}", channel.to_string());
}
