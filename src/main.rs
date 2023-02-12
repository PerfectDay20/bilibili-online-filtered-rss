extern crate core;

use std::sync::Arc;

use clap::Parser;
use tokio::sync::RwLock;
use tracing_subscriber::fmt::format::FmtSpan;
use warp::Filter;

use bilibili::blacklist;
use bilibili::blacklist::Blacklist;

use crate::cli::Cli;

mod bilibili;
mod ddys;
mod cli;
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

    let blacklist = Arc::new(RwLock::new(Blacklist::from(cli.blacklist_path)));
    let blacklist_filter = warp::any().map(move || Arc::clone(&blacklist));

    // GET /
    let get_rss = warp::get()
        .and(warp::path::end())
        .and(blacklist_filter.clone())
        .and_then(bilibili::rss_generator::generate_rss);


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
        .and_then(blacklist::patch_blacklist);

    // PUT /blacklist
    let put_blacklist = warp::put()
        .and(warp::path("blacklist"))
        .and(warp::path::end())
        .and(blacklist_filter.clone())
        .and(warp::body::content_length_limit(32 * 1024))
        .and(warp::body::json())
        .and_then(blacklist::put_blacklist);

    // GET /ddys
    let get_ddys = warp::get()
        .and(warp::path("ddys"))
        .and(warp::path::end())
        .and_then(ddys::rss_generator::generate_rss);

    let routes = get_rss
        .or(get_blacklist)
        .or(patch_blacklist)
        .or(put_blacklist)
        .or(get_ddys)
        .with(warp::trace::request())
        .recover(error::return_error);

    warp::serve(routes).run((cli.host, cli.port)).await;
}
