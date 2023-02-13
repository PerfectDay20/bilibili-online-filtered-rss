use rss::{ChannelBuilder, ImageBuilder, Item, ItemBuilder};
use rss::validation::Validate;
use warp::{Rejection, Reply};
use scraper::{Html, Selector};

use crate::ddys::Ddys;
use crate::error::MyError;

const TITLE: &str = "ddys.site";
const LINK: &str = "https://ddys.site";
const DESC: &str = "A rss for ddys";
const ICON_URL: &str = "https://ddys.art/favicon-32x32.png";

fn create_rss(items: Vec<Ddys>) -> Result<impl Reply, Rejection> {
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
                    .link(d.url.to_string())
                    .build()
            }).collect::<Vec<Item>>()
        )
        .build();

    channel.validate().map_err(MyError::Validation)?;
    Ok(warp::reply::with_header(channel.to_string(), "content-type", "text/xml; charset=utf-8"))
}

fn create_item_desc(d: &Ddys) -> String {
    format!(r#"
    <b>category:</b> {category}
    <p></p>
    <b>desc:</b> {desc}
    <p></p>
    <img style="width:100%" src="{img_src}" width="500">"#,
            category = d.category.join(" "),
            desc = d.desc,
            img_src = d.image_url)
}

pub async fn generate_rss() -> Result<impl Reply, Rejection> {
    let html = reqwest::get("https://ddys.pro")
        .await.map_err(MyError::Reqwest)?
        .text()
        .await.map_err(MyError::Reqwest)?;

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
                ddys.url = titles.select(&url_selector).next().unwrap().value().attr("href").unwrap().to_string();
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

    create_rss(result)
}
