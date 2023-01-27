extern crate core;

use std::error::Error;
use std::net::SocketAddr;

use clap::Parser;
use env_logger::Env;
use hyper::{Body, Request, Response, Server};
use hyper::{Method, StatusCode};
use hyper::header::CONTENT_TYPE;
use hyper::service::{make_service_fn, service_fn};
use log::{error, info};
use serde_json::json;
use tokio::sync::{mpsc, oneshot};
use tokio::sync::mpsc::Sender;

use bilibili::{Bili, BiliData};
use blacklist::Blacklist;

use crate::cli::Cli;
use crate::Command::{AddBlacklist, GetBlacklist, GetRss, ReplaceBlacklist};

mod bilibili;
mod rss_generator;
mod blacklist;
mod cli;

async fn call_api_generate_rss(blacklist: &Blacklist) -> Result<String, Box<dyn Error + Send + Sync>> {
    let resp = reqwest::get("https://api.bilibili.com/x/web-interface/online/list")
        .await?
        .json::<Bili>()
        .await?;

    let items: Vec<BiliData> = resp.data.into_iter()
        .filter(|bili_data| blacklist.filter(bili_data))
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
            let cmd = GetBlacklist { responder: one_tx };
            tx.send(cmd).await;

            match one_rx.await {
                Ok(s) => {
                    *response.body_mut() = Body::from(s);
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
            let full_body = hyper::body::to_bytes(req.into_body()).await?;

            match serde_json::from_slice::<Blacklist>(&full_body.to_vec()) {
                Ok(new_blacklist) => {
                    let (one_tx, one_rx) = oneshot::channel();
                    let cmd = AddBlacklist { new_blacklist, responder: one_tx };
                    tx.send(cmd).await;

                    match one_rx.await {
                        Ok(s) => {
                            response.headers_mut().insert(CONTENT_TYPE, "application/json".parse().unwrap());
                            *response.body_mut() = Body::from(s);
                        }
                        Err(_) => { *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR; }
                    }
                }
                Err(e) => {
                    *response.status_mut() = StatusCode::BAD_REQUEST;
                    let error_message =
                        String::from(r#"blacklist should be a json object like {"authors":["foo"], categories:["bar"]} "#)
                            + "\n"
                            + &e.to_string();
                    *response.body_mut() = Body::from(error_message);
                }
            }
        }

        // Replace new items to blacklist
        (&Method::PUT, "/blacklist") => {
            let full_body = hyper::body::to_bytes(req.into_body()).await?;
            match serde_json::from_slice::<Blacklist>(&full_body.to_vec()) {
                Ok(new_blacklist) => {
                    response.headers_mut().insert(CONTENT_TYPE, "application/json".parse().unwrap());
                    let (one_tx, one_rx) = oneshot::channel();
                    let cmd = ReplaceBlacklist { new_blacklist, responder: one_tx };
                    tx.send(cmd).await;

                    match one_rx.await {
                        Ok(s) => { *response.body_mut() = Body::from(s); }
                        Err(_) => { *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR; }
                    }
                }
                Err(e) => {
                    *response.status_mut() = StatusCode::BAD_REQUEST;
                    let error_message =
                        String::from(r#"blacklist should be a json object like {"authors":["foo"], categories:["bar"]} "#)
                            + "\n"
                            + &e.to_string();
                    *response.body_mut() = Body::from(error_message);
                }
            }
        }

        _ => {
            *response.status_mut() = StatusCode::NOT_FOUND;
        }
    };

    Ok(response)
}


type Responder<T> = oneshot::Sender<T>;

/// Http requests will create and send these commands to worker
#[derive(Debug)]
enum Command {
    GetRss { responder: Responder<Result<String, Box<dyn Error + Send + Sync>>> },
    GetBlacklist { responder: Responder<String> },
    AddBlacklist { new_blacklist: Blacklist, responder: Responder<String> },
    ReplaceBlacklist { new_blacklist: Blacklist, responder: Responder<String> },
}


#[tokio::main]
async fn main() {
    // Set default log level to info
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let cli = Cli::parse();

    let (tx, mut rx) = mpsc::channel::<Command>(100);

    // The worker holds the blacklist and executes commands created by http requests
    let worker = tokio::spawn(async move {
        let mut blacklist = blacklist::create_blacklist(cli.blacklist_path);
        while let Some(cmd) = rx.recv().await {
            match cmd {
                GetRss { responder } => {
                    let result = call_api_generate_rss(&blacklist).await;
                    if let Err(_) = responder.send(result) {
                        error!("the sender dropped")
                    }
                }
                GetBlacklist { responder } => {
                    if let Err(_) = responder.send(blacklist.to_json()) {
                        error!("the sender dropped")
                    }
                }
                AddBlacklist { new_blacklist, responder } => {
                    info!("add items to blacklist: {}", new_blacklist.to_json());
                    blacklist.extend(Some(new_blacklist));
                    if let Err(_) = responder.send(blacklist.to_json()) {
                        error!("the sender dropped")
                    }
                }
                ReplaceBlacklist { new_blacklist, responder } => {
                    blacklist = new_blacklist;
                    info!("replace blacklist to: {}", blacklist.to_json());
                    if let Err(_) = responder.send(blacklist.to_json()) {
                        error!("the sender dropped");
                    }
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

    let addr = SocketAddr::from((cli.host, cli.port));
    let server = Server::bind(&addr).serve(make_svc);

    if let Err(e) = server.await {
        error!("server error: {}", e);
    }
    worker.await.unwrap();
}
