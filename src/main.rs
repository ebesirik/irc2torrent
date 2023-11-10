mod torrent_processor;
mod irc_processor;
mod command_processor;
mod config;
// mod pub_sub;
// pub use pub_sub::{Publisher, Subscriber};
extern crate pub_sub;
extern crate syslog;
#[macro_use]
extern crate log;

use serde::{Serialize, Deserialize};
use std::{process};
use toml;
use regex::Regex;
use directories::BaseDirs;
use syslog::{Facility, Formatter3164, BasicLogger};
use log::{LevelFilter};
use crate::command_processor::CommandProcessor;
use crate::irc_processor::IrcProcessor;
use crate::torrent_processor::TorrentProcessor;
use crate::config::Config;

static IRC_CONFIG_FILE: &str = "irc.toml";
static OPTIONS_CONFIG_FILE: &str = "options.toml";

#[tokio::main]
async fn main() -> Result<(), failure::Error> {
    let formatter = Formatter3164 {
        facility: Facility::LOG_USER,
        hostname: None,
        process: "irc2torrent".into(),
        pid: process::id(),
    };
    let logger = syslog::unix(formatter).expect("could not connect to syslog");
    let _ = log::set_boxed_logger(Box::new(BasicLogger::new(logger)))
        .map(|()| log::set_max_level(LevelFilter::Info));
    info!("Started the app");

    let re: Regex = Regex::new(r".*Name:'(?P<name>.*)' uploaded by.*https://www.torrentleech.org/torrent/(?P<id>\d+)").unwrap();

    if let Ok(options) = Config::new().await {//Some(proj_dir) = BaseDirs::new()
        let irc_config = options.get_irc_config();
        if let Some(proj_dir) = BaseDirs::new() {
            let mut processor = TorrentProcessor::new(options.get_rss_key(), options.get_xmlrpc_url(), proj_dir.config_dir().to_path_buf(), &options);
            let cp = CommandProcessor::new(&options, &mut processor, re.clone());
            let irc = IrcProcessor::new(&irc_config, re, &mut processor, &cp);
            irc.start_listening().await;
        }
    }

    Ok(())
}


