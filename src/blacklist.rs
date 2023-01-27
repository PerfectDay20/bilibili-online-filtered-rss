use std::{fs, process};
use std::collections::HashSet;
use std::path::PathBuf;

use log::{error, info};
use serde::Deserialize;
use serde::Serialize;

use crate::bilibili::BiliData;

pub fn create_blacklist(path: Option<PathBuf>) -> Blacklist {
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
            info!("no blacklist path provided, won't enable filter");
            Blacklist { authors: HashSet::new(), categories: HashSet::new() }
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Blacklist {
    #[serde(default)]
    authors: HashSet<String>,
    #[serde(default)]
    categories: HashSet<String>,
}

impl Blacklist {
    /// Filter rss content based on author name and category.
    /// Return true when items can be read
    pub fn filter(&self, bili_data: &BiliData) -> bool {
        !self.authors.contains(&bili_data.owner.name)
            && !self.categories.contains(&bili_data.tname)
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

impl Extend<Blacklist> for Blacklist {
    fn extend<T: IntoIterator<Item=Blacklist>>(&mut self, iter: T) {
        for b in iter {
            self.authors.extend(b.authors);
            self.categories.extend(b.categories);
        }
    }
}
