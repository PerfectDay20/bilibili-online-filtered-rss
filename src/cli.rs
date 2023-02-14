use std::net::IpAddr;
use std::path::PathBuf;

use clap::Parser;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[arg(long, default_value_t = IpAddr::from([127, 0, 0, 1]))]
    pub host: IpAddr,
    #[arg(short, long, default_value_t = 3000)]
    pub port: u16,
    #[arg(long, default_value_t = false)]
    pub disable_blacklist: bool,
    #[arg(short, long, value_name = "FILE")]
    pub blacklist_path: Option<PathBuf>,
}
