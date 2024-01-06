use std::sync::{Arc, Mutex};

use regex::Regex;

use crate::command_processor::commands::CommandProcessor;
use crate::config::config::Config;
use crate::irc_processor::irc::IrcProcessor;
use crate::torrent_processor::torrent::TorrentProcessor;

mod irc_processor;
mod command_processor;
mod torrent_processor;
mod config;

static IRC_CONFIG_FILE: &str = "irc.toml";
static OPTIONS_CONFIG_FILE: &str = "options.toml";


pub struct Irc2Torrent {
    config: Arc<Mutex<Config>>,
    torrent_processor: Arc<Mutex<TorrentProcessor>>,
    command_processor: Arc<Mutex<CommandProcessor>>,
    irc_processor: Arc<Mutex<IrcProcessor>>,
}

impl Irc2Torrent {
    pub async fn new() -> Self {
        let torrent = pub_sub::PubSub::new();
        let torrent_ch = torrent.clone();
        let commands = pub_sub::PubSub::new();
        let command_ch = commands.clone();
        let irc = pub_sub::PubSub::new();
        let irc_ch = irc.clone();
        let cfg = Config::new().await.unwrap();
        let config = Arc::new(Mutex::new(cfg));
        let re: Regex = Regex::new(r".*Name:'(?P<name>.*)' uploaded by.*https://www.torrentleech.org/torrent/(?P<id>\d+)").unwrap();
        let torrent_processor = Arc::new(Mutex::new(
            TorrentProcessor::new(Default::default(), config.clone(), torrent_ch, vec![commands.clone().subscribe(), irc.clone().subscribe()])));
        let command_processor = Arc::new(Mutex::new(
            CommandProcessor::new(config.clone(), torrent_processor.clone(), re.clone(), command_ch, vec![torrent.clone().subscribe(), irc.clone().subscribe()])));
        let irc_processor = Arc::new(Mutex::new(
            IrcProcessor::new(config.clone(), re.clone(), torrent_processor.clone(), command_processor.clone(), irc_ch, vec![torrent.clone().subscribe(), commands.clone().subscribe()])));
        Self { config, torrent_processor, command_processor, irc_processor }
    }

    pub async fn start(&self) {
        self.irc_processor.lock().unwrap().start_listening().await;
    }
}
