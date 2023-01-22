use std::collections::HashSet;
use std::net::SocketAddr;
use std::sync::Arc;

use hyper::{Body, Request, Response, Server};
use hyper::{Method, StatusCode};
use hyper::header::{CONTENT_ENCODING, CONTENT_TYPE};
use hyper::service::{make_service_fn, service_fn};

use bilibili::{Bili, Data};

mod bilibili;
mod rss_generator;
mod blacklist;


async fn call_api_generate_rss(blacklist: &HashSet<String>) -> Result<String, Box<dyn std::error::Error>> {
    let resp = reqwest::get("https://api.bilibili.com/x/web-interface/online/list")
        .await?
        .json::<Bili>()
        .await?;

    // let blacklist: HashSet<String> = blacklist::create_blacklist();

    let items: Vec<Data> = resp.data.into_iter().filter(|d| !blacklist.contains(&d.owner.name))
        .collect();

    Ok(rss_generator::create_rss(items)?)
}

async fn process(req: Request<Body>, blacklist: Arc<HashSet<String>>) -> Result<Response<Body>, hyper::Error> {
    let mut response = Response::new(Body::empty());

    match (req.method(), req.uri().path()) {
        (&Method::GET, "/") => {
            response.headers_mut().insert(CONTENT_TYPE, "application/xml".parse().unwrap());
            response.headers_mut().insert(CONTENT_ENCODING, "utf-8".parse().unwrap());
            *response.body_mut() = Body::from(call_api_generate_rss(&blacklist).await.unwrap());
        }
        _ => {
            *response.status_mut() = StatusCode::NOT_FOUND;
        }
    };

    Ok(response)
}


#[tokio::main]
async fn main() {
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));

    let blacklist = Arc::new(blacklist::create_blacklist());

    let make_svc = make_service_fn(
        move |_conn| {
            // https://stackoverflow.com/questions/67960931/moving-non-copy-variable-into-async-closure-captured-variable-cannot-escape-fn
            let b1 = Arc::clone(&blacklist);
            async {
                Ok::<_, hyper::Error>(service_fn(move |req| {
                    process(req, Arc::clone(&b1))
                }))
            }
        }
    );

    let server = Server::bind(&addr).serve(make_svc);

    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}
