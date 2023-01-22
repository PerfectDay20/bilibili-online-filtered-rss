use std::collections::HashSet;
use std::fs;

use log::info;

pub fn create_blacklist() -> HashSet<String> {
    let s = fs::read_to_string("resources/blacklist.txt").unwrap();
    let set: HashSet<String> = s.split('\n').filter(|x| *x!="").map(|x| x.to_string()).collect();
    info!("init blacklist: {:?}", set);
    set
}
