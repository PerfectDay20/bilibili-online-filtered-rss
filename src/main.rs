extern crate core;

use std::sync::Arc;

use clap::Parser;
use futures::future;
use tokio::sync::RwLock;
use tracing::info;
use tracing_subscriber::fmt::format::FmtSpan;
use warp::{reject, Filter};

use bilibili::blacklist;
use bilibili::blacklist::Blacklist;

use crate::cache::RssCache;
use crate::cli::Cli;
use crate::error::MyError;

mod bilibili;
mod cache;
mod cli;
mod ddys;
mod error;

#[tokio::main]
async fn main() {
    let log_filter = std::env::var("RUST_LOG")
        .unwrap_or_else(|_| "bilibili_online_rss=info,warp=info".to_string()); // snake case here
    tracing_subscriber::fmt()
        .with_env_filter(log_filter)
        .with_span_events(FmtSpan::CLOSE)
        .init();

    let cli = Cli::parse();

    if cli.auth_password.is_none() {
        info!("User didn't set auth_password, the update blacklist API will not work");
    }

    let blacklist = if cli.disable_blacklist {
        info!("blacklist is disabled");
        Arc::new(RwLock::new(Blacklist::default()))
    } else {
        Arc::new(RwLock::new(Blacklist::from(cli.blacklist_path)))
    };

    let blacklist_filter = warp::any().map(move || Arc::clone(&blacklist));

    let cache = Arc::new(RwLock::new(RssCache::new()));
    let cache_filter = warp::any().map(move || Arc::clone(&cache));

    let check_update_api_filter = warp::any()
        .and(warp::header::<String>("Authorization"))
        .and_then(move |auth_header: String| match cli.auth_password.clone() {
            None => future::err(reject::custom(MyError::AuthNotSet)),
            Some(p) => {
                if auth_header.eq(&p) {
                    future::ok(())
                } else {
                    future::err(reject::custom(MyError::UnAuthorized))
                }
            }
        })
        .untuple_one();

    // GET /bilibili/feed
    let get_rss = warp::get()
        .and(warp::path!("bilibili" / "feed"))
        .and(blacklist_filter.clone())
        .and(cache_filter.clone())
        .and_then(bilibili::rss_generator::generate_rss);

    // GET /bilibili/blacklist
    let get_blacklist = warp::get()
        .and(warp::path!("bilibili" / "blacklist"))
        .and(blacklist_filter.clone())
        .then(|b: Arc<RwLock<Blacklist>>| async move { warp::reply::json(&*b.read().await) });

    // PATCH /bilibili/blacklist
    let patch_blacklist = warp::patch()
        .and(check_update_api_filter.clone())
        .and(warp::path!("bilibili" / "blacklist"))
        .and(blacklist_filter.clone())
        .and(warp::body::content_length_limit(32 * 1024))
        .and(warp::body::json())
        .and_then(blacklist::patch_blacklist);

    // PUT /bilibili/blacklist
    let put_blacklist = warp::put()
        .and(check_update_api_filter.clone())
        .and(warp::path!("bilibili" / "blacklist"))
        .and(blacklist_filter.clone())
        .and(warp::body::content_length_limit(32 * 1024))
        .and(warp::body::json())
        .and_then(blacklist::put_blacklist);

    // GET /status
    let get_status = warp::get().and(warp::path!("status")).map(|| "ok");

    // GET /ddys/feed
    let get_ddys = warp::get()
        .and(warp::path!("ddys" / "feed"))
        .and(cache_filter.clone())
        .and_then(ddys::rss_generator::generate_rss);

    let routes = get_rss
        .or(get_blacklist)
        .or(patch_blacklist)
        .or(put_blacklist)
        .or(get_status)
        .or(get_ddys)
        .with(warp::trace::request())
        .recover(error::return_error);

    warp::serve(routes).run((cli.host, cli.port)).await;
}
