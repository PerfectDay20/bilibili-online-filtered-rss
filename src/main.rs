extern crate core;

use std::sync::Arc;

use clap::Parser;
use tokio::sync::RwLock;
use tracing_subscriber::fmt::format::FmtSpan;
use warp::Filter;

use blacklist::Blacklist;

use crate::cli::Cli;

mod bilibili;
mod blacklist;
mod cli;
mod rss_generator;
mod error;

#[tokio::main]
async fn main() {
    let log_filter = std::env::var("RUST_LOG")
        .unwrap_or_else(|_| "bilibili-online-rss=info,warp=info".to_string());
    tracing_subscriber::fmt().with_env_filter(log_filter)
        .with_span_events(FmtSpan::CLOSE)
        .init();


    let cli = Cli::parse();

    let blacklist = Arc::new(RwLock::new(Blacklist::from(cli.blacklist_path)));
    let blacklist_filter = warp::any().map(move || Arc::clone(&blacklist));

    // GET /
    let get_rss = warp::get()
        .and(warp::path::end())
        .and(blacklist_filter.clone())
        .and_then(rss_generator::generate_rss);


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

    let routes = get_rss
        .or(get_blacklist)
        .or(patch_blacklist)
        .or(put_blacklist)
        .with(warp::trace::request())
        .recover(error::return_error);

    warp::serve(routes).run((cli.host, cli.port)).await;
}
