use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::{fs, process};

use serde::Deserialize;
use serde::Serialize;
use tokio::sync::RwLock;
use tracing::{error, info};
use warp::{Rejection, Reply};

use crate::bilibili::BiliData;

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Blacklist {
    #[serde(default = "Blacklist::default_enable")]
    enable: bool,
    authors: HashSet<String>,
    categories: HashSet<String>,
}

impl Blacklist {
    fn default_enable() -> bool {
        true
    }
    /// Filter rss content based on author name and category.
    /// Return true when items can be read
    pub fn filter(&self, bili_data: &BiliData) -> bool {
        if self.enable {
            !self.authors.contains(&bili_data.owner.name)
                && !self.categories.contains(&bili_data.tname)
        } else {
            true
        }
    }
}

impl From<Option<PathBuf>> for Blacklist {
    fn from(path: Option<PathBuf>) -> Self {
        match path {
            Some(p) => {
                info!("use blacklist at: {}", p.to_str().unwrap());
                match fs::read_to_string(p) {
                    Ok(s) => {
                        let blacklist: Blacklist = serde_json::from_str(&s).unwrap();
                        info!("init blacklist: {blacklist:?}");
                        blacklist
                    }
                    Err(e) => {
                        // Can't read file, the config is not valid, exit now
                        error!("fail to read blacklist config file: {}", e.to_string());
                        process::exit(1);
                    }
                }
            }
            None => {
                info!("no blacklist path provided, use default");
                let blacklist: Blacklist =
                    serde_json::from_str(include_str!("../../resources/blacklist.json")).unwrap();
                info!("init blacklist: {blacklist:?}");
                blacklist
            }
        }
    }
}

impl Extend<Blacklist> for Blacklist {
    fn extend<T: IntoIterator<Item = Blacklist>>(&mut self, iter: T) {
        for b in iter {
            self.authors.extend(b.authors);
            self.categories.extend(b.categories);
        }
    }
}

pub async fn patch_blacklist(
    blacklist: Arc<RwLock<Blacklist>>,
    body: Blacklist,
) -> Result<impl Reply, Rejection> {
    info!("{body:?}");
    let mut b = blacklist.write().await;
    b.extend(Some(body));
    Ok(format!("added: {b:?}"))
}

pub async fn put_blacklist(
    blacklist: Arc<RwLock<Blacklist>>,
    body: Blacklist,
) -> Result<impl Reply, Rejection> {
    info!("{body:?}");
    let mut b = blacklist.write().await;
    *b = body;
    Ok(format!("replaced: {b:?}"))
}
