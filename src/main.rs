extern crate core;

use std::borrow::BorrowMut;
use std::collections::HashSet;
use std::error::Error;
use std::net::SocketAddr;

use env_logger::Env;
use hyper::{Body, Request, Response, Server};
use hyper::{Method, StatusCode};
use hyper::header::{CONTENT_TYPE};
use hyper::service::{make_service_fn, service_fn};
use log::info;
use serde_json::json;
use tokio::sync::{mpsc, oneshot};
use tokio::sync::mpsc::Sender;

use bilibili::{Bili, Data};

use crate::Command::{AddBlacklist, GetBlacklist, GetRss, ReplaceBlacklist};

mod bilibili;
mod rss_generator;
mod blacklist;

async fn call_api_generate_rss(blacklist: &HashSet<String>) -> Result<String, Box<dyn Error + Send + Sync>> {
    let resp = reqwest::get("https://api.bilibili.com/x/web-interface/online/list")
        .await?
        .json::<Bili>()
        .await?;

    let items: Vec<Data> = resp.data.into_iter().filter(|d| !blacklist.contains(&d.owner.name))
        .collect();

    Ok(rss_generator::create_rss(items)?)
}

async fn process(req: Request<Body>, tx: Sender<Command>) -> Result<Response<Body>, hyper::Error> {
    let mut response = Response::new(Body::empty());
    match (req.method(), req.uri().path()) {
        // Get rss content
        (&Method::GET, "/") => {
            response.headers_mut().insert(CONTENT_TYPE, "application/xml".parse().unwrap());

            let result = tokio::spawn(async move {
                let (one_tx, one_rx) = oneshot::channel();
                let cmd = GetRss { responder: one_tx };
                tx.send(cmd).await;

                match one_rx.await {
                    Ok(r) => {
                        match r {
                            Ok(body) => Ok(body),
                            Err(e) => Err(e.to_string()),
                        }
                    }
                    Err(e) => Err(e.to_string()), // TODO: how to handle this error?
                }
            }).await;

            match result {
                Ok(inner_result) => {
                    match inner_result {
                        Ok(body) => {
                            *response.body_mut() = Body::from(body);
                        }
                        Err(e) => {
                            *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                            *response.body_mut() = Body::from(e.to_string());
                        }
                    }
                }
                Err(e) => {
                    *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                    *response.body_mut() = Body::from(e.to_string());
                }
            }
        }

        // Get blacklist content
        (&Method::GET, "/blacklist") => {
            response.headers_mut().insert(CONTENT_TYPE, "application/json".parse().unwrap());
            let (one_tx, one_rx) = oneshot::channel();
            let cmd = Command::GetBlacklist { responder: one_tx };
            tx.send(cmd).await;

            match one_rx.await {
                Ok(v) => {
                    let body = json!(v).to_string();
                    *response.body_mut() = Body::from(body);
                }
                Err(e) => {
                    let body = json!({
                        "error": e.to_string()
                    }).to_string();
                    *response.body_mut() = Body::from(body);
                }
            }
        }

        // Add new items to blacklist
        (&Method::PATCH, "/blacklist") => {
            response.headers_mut().insert(CONTENT_TYPE, "application/json".parse().unwrap());

            let full_body = hyper::body::to_bytes(req.into_body()).await?;
            let items: Vec<String> = serde_json::from_slice(&full_body.to_vec()).unwrap();

            let (one_tx, one_rx) = oneshot::channel();
            let cmd = AddBlacklist { items, responder: one_tx };
            tx.send(cmd).await;

            match one_rx.await {
                Ok(s) => { *response.body_mut() = Body::from(s);}
                Err(_) => { *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;}
            }
        }

        // Replace new items to blacklist
        (&Method::PUT, "/blacklist") => {
            response.headers_mut().insert(CONTENT_TYPE, "application/json".parse().unwrap());

            let full_body = hyper::body::to_bytes(req.into_body()).await?;
            let items: Vec<String> = serde_json::from_slice(&full_body.to_vec()).unwrap();

            let (one_tx, one_rx) = oneshot::channel();
            let cmd = ReplaceBlacklist { items, responder: one_tx };
            tx.send(cmd).await;

            match one_rx.await {
                Ok(s) => { *response.body_mut() = Body::from(s);}
                Err(_) => { *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;}
            }
        }

        _ => {
            *response.status_mut() = StatusCode::NOT_FOUND;
        }
    };

    Ok(response)
}


type Responder<T> = oneshot::Sender<T>;

#[derive(Debug)]
enum Command {
    GetRss { responder: Responder<Result<String, Box<dyn Error + Send + Sync>>> },
    GetBlacklist { responder: Responder<Vec<String>> },
    AddBlacklist { items: Vec<String>, responder: Responder<String> },
    ReplaceBlacklist {items: Vec<String>, responder: Responder<String> },
}


#[tokio::main]
async fn main() {
    // Set default log level to info
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));

    let (tx, mut rx) = mpsc::channel::<Command>(100);
    let worker = tokio::spawn(async move {
        let mut blacklist = blacklist::create_blacklist();
        while let Some(cmd) = rx.recv().await {
            match cmd {
                GetRss { responder } => {
                    let result = call_api_generate_rss(&blacklist).await;
                    responder.send(result);
                }
                GetBlacklist { responder } => {
                    responder.send(blacklist.iter().map(|s| s.to_owned()).collect());
                }
                AddBlacklist { items, responder } => {
                    blacklist.extend(items);
                    responder.send(String::from("added"));
                }
                ReplaceBlacklist {items, responder} => {
                    blacklist = items.into_iter().collect();
                    info!("replace blacklist to: {:?}", blacklist);
                    responder.send(String::from("replaced"));
                }
            }
        }
    });


    let make_svc = make_service_fn(
        move |_conn| {
            let tx = tx.clone();
            async {
                Ok::<_, hyper::Error>(service_fn(move |req| process(req, tx.clone())))
            }
        }
    );

    let server = Server::bind(&addr).serve(make_svc);

    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
    worker.await.unwrap();
}
