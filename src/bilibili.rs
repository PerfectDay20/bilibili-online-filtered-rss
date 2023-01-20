use serde::Deserialize;

#[derive(Deserialize)]
pub struct Bili {
    pub data: Vec<Data>,
}

#[derive(Deserialize)]
pub struct Data {
    pub tname: String,
    pub pic: String,
    pub title: String,
    pub owner: Owner,
    pub desc: String,
    pub stat: Stat,
    pub short_link: String,
}

#[derive(Deserialize)]
pub struct Owner {
    pub name: String,
}

#[derive(Deserialize)]
pub struct Stat {
    pub view: u32,
    pub danmaku: u32,
}
