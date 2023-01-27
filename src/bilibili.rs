use serde::Deserialize;

#[derive(Deserialize)]
pub struct Bili {
    pub data: Vec<BiliData>,
}

#[derive(Deserialize)]
pub struct BiliData {
    pub tname: String, // category
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
