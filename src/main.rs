extern crate core;

use std::sync::Arc;

use clap::Parser;
use env_logger::Env;
use log::info;
use reqwest::StatusCode;
use tokio::sync::RwLock;
use warp::{Filter, Rejection, Reply};
use warp::body::BodyDeserializeError;

use bilibili::{Bili, BiliData};
use blacklist::Blacklist;
use error::InternalError;

use crate::cli::Cli;

mod bilibili;
mod blacklist;
mod cli;
mod rss_generator;
mod error;

#[tokio::main]
async fn main() {
    // Set default log level to info
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    let cli = Cli::parse();

    let blacklist = Arc::new(RwLock::new(Blacklist::from(cli.blacklist_path)));
    let blacklist_filter = warp::any().map(move || Arc::clone(&blacklist));

    // GET /
    let get_rss = warp::get()
        .and(warp::path::end())
        .and(blacklist_filter.clone())
        .and_then(generate_rss);


    // GET /blacklist
    let get_blacklist = warp::get()
        .and(warp::path("blacklist"))
        .and(warp::path::end())
        .and(blacklist_filter.clone())
        .then(|b: Arc<RwLock<Blacklist>>| async move {
            warp::reply::json(&*b.read().await)
        });

    // PATCH /blacklist
    let patch_blacklist = warp::patch()
        .and(warp::path("blacklist"))
        .and(warp::path::end())
        .and(blacklist_filter.clone())
        .and(warp::body::content_length_limit(32 * 1024))
        .and(warp::body::json())
        .and_then(patch_blacklist);

    // PUT /blacklist
    let put_blacklist = warp::put()
        .and(warp::path("blacklist"))
        .and(warp::path::end())
        .and(blacklist_filter.clone())
        .and(warp::body::content_length_limit(32 * 1024))
        .and(warp::body::json())
        .and_then(put_blacklist);


    let routes = get_rss
        .or(get_blacklist)
        .or(patch_blacklist)
        .or(put_blacklist)
        .recover(return_error);

    warp::serve(routes).run((cli.host, cli.port)).await;
}

async fn return_error(r: Rejection) -> Result<impl Reply, Rejection> {
    info!("{:?}", r);
    if let Some(e) = r.find::<BodyDeserializeError>() {
        Ok(warp::reply::with_status(e.to_string(), StatusCode::UNPROCESSABLE_ENTITY))
    } else {
        Err(warp::reject())
    }
}

async fn patch_blacklist(blacklist: Arc<RwLock<Blacklist>>, body: Blacklist) -> Result<impl Reply, Rejection> {
    let mut b = blacklist.write().await;
    b.extend(Some(body));
    Ok("added")
}

async fn put_blacklist(blacklist: Arc<RwLock<Blacklist>>, body: Blacklist) -> Result<impl Reply, Rejection> {
    let mut b = blacklist.write().await;
    *b = body;
    Ok("replaced")
}

async fn call_api() -> Result<Bili, InternalError> {
    let resp = reqwest::get("https://api.bilibili.com/x/web-interface/online/list")
        .await?
        .json::<Bili>()
        .await?;

    Ok(resp)
}

async fn generate_rss(blacklist: Arc<RwLock<Blacklist>>) -> Result<impl Reply, Rejection> {
    let resp = call_api().await?;

    let b = blacklist.read().await;
    let items: Vec<BiliData> = resp.data
        .into_iter()
        .filter(move |bili_data| b.filter(bili_data))
        .collect();

    Ok(rss_generator::create_rss(items)?)
}
