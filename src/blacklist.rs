use std::collections::HashSet;
use std::fs;

use log::info;
use serde::Deserialize;
use serde::Serialize;

use crate::bilibili::BiliData;

pub fn create_blacklist() -> Blacklist {
    let s = fs::read_to_string("resources/blacklist.json").unwrap();
    let blacklist: Blacklist = serde_json::from_str(&s).unwrap();
    info!("init blacklist: {blacklist:?}");
    blacklist
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Blacklist {
    authors: Option<HashSet<String>>,
    categories: Option<HashSet<String>>,
}

impl Blacklist {
    /// Filter rss content based on author name and category.
    /// Return true when items can be read
    pub fn filter(&self, bili_data: &BiliData) -> bool {
        if let Some(set) = &self.authors {
            if set.contains(&bili_data.owner.name) {
                return false;
            }
        }

        if let Some(set) = &self.categories {
            if set.contains(&bili_data.tname) {
                return false;
            }
        }

        true
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

impl Extend<Blacklist> for Blacklist {
    fn extend<T: IntoIterator<Item=Blacklist>>(&mut self, iter: T) {
        let mut authors = HashSet::new();
        let mut categories = HashSet::new();

        if let Some(set) = &self.authors {
            authors.extend(set.iter().map(|s|s.to_owned()));
        }

        if let Some(set) = &self.categories {
            categories.extend(set.iter().map(|s| s.to_owned()));
        }

        for b in iter {
            authors.extend(b.authors.unwrap_or_default());
            categories.extend(b.categories.unwrap_or_default());
        }
        self.authors = Some(authors);
        self.categories = Some(categories);
    }
}
