use std::collections::HashSet;
use std::fs;

pub fn create_blacklist() -> HashSet<String> {
    let s = fs::read_to_string("resources/blacklist.txt").unwrap();
    let set: HashSet<String> = s.split('\n').filter(|x| *x!="").map(|x| x.to_string()).collect();
    println!("init blacklist: {:?}", set);
    set
}
