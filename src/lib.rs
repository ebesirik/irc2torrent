use std::sync::{Arc, Mutex};

use crate::clients::{TorrentClientsEnum};
use crate::clients::flood::Flood;
use crate::clients::rtorrent::rTorrent;
use crate::command_processor::commands::CommandProcessor;
use crate::config::config::{Config, TorrentClientOption, TorrentPlatforms};
use crate::irc_processor::irc::IrcProcessor;
use crate::platforms::{TorrentPlatform, TorrentPlatformsEnum};
use crate::platforms::tl::TorrentLeech;
use crate::torrent_processor::torrent::TorrentProcessor;

mod irc_processor;
mod command_processor;
mod torrent_processor;
mod config;
mod clients;
mod platforms;

static IRC_CONFIG_FILE: &str = "irc.toml";
static OPTIONS_CONFIG_FILE: &str = "options.toml";


pub struct Irc2Torrent {
    config: Arc<Mutex<Config>>,
    torrent_processor: Arc<Mutex<TorrentProcessor>>,
    command_processor: Box<Arc<Mutex<CommandProcessor>>>,
    irc_processor: Box<Arc<Mutex<IrcProcessor>>>,
}

impl Irc2Torrent {
    pub async fn new() -> Self {
        let torrent = pub_sub::PubSub::new();
        let torrent_ch = torrent.clone();
        let commands = pub_sub::PubSub::new();
        let command_ch = commands.clone();
        let irc = pub_sub::PubSub::new();
        let irc_ch = irc.clone();
        let mut cfg = Config::new().await.unwrap();
        let mut torrent_client =
            Irc2Torrent::get_torrent_client(&mut cfg.get_torrent_client())
                .await;
        let mut torrent_platform = match cfg.get_torrent_platform() {
            TorrentPlatforms::TorrentLeech(ref c) => {
                TorrentPlatformsEnum::TorrentLeech(TorrentLeech::new(c.rss_key.clone(), c.torrent_dir.clone()))
            }
        };
        let config = Arc::new(Mutex::new(cfg));
        // let re: Regex = Regex::new(r".*Name:'(?P<name>.*)' uploaded by.*https://www.torrentleech.org/torrent/(?P<id>\d+)").unwrap();
        let torrent_processor = Arc::new(Mutex::new(
            TorrentProcessor::new(config.clone(), torrent_ch, vec![commands.clone().subscribe(), irc.clone().subscribe()], torrent_client, torrent_platform)));
        let command_processor = Arc::new(Mutex::new(
            CommandProcessor::new(config.clone(), torrent_processor.clone(), command_ch, vec![torrent.clone().subscribe(), irc.clone().subscribe()])));
        let irc_processor = Arc::new(Mutex::new(
            IrcProcessor::new(config.clone(), torrent_processor.clone(), command_processor.clone(), irc_ch, vec![torrent.clone().subscribe(), commands.clone().subscribe()])));
        Self { config, torrent_processor, command_processor: Box::new(command_processor), irc_processor: Box::new(irc_processor) }
    }

    pub async fn start(&self, irc_processor: Arc<Mutex<IrcProcessor>>) {
        irc_processor.lock().unwrap().start_listening().await;
    }

    async fn get_torrent_client(clients: &mut TorrentClientOption) -> TorrentClientsEnum {
        match clients {
            TorrentClientOption::rTorrent(ref mut c) => {
                return TorrentClientsEnum::Rtorrent(rTorrent::new(c.xmlrpc_url.clone()).await.unwrap());
            }
            TorrentClientOption::Flood(ref mut c) => {
                return TorrentClientsEnum::Flood(Flood::new(
                    c.username.clone(),
                    c.password.clone(),
                    c.url.clone(),
                    c.destination.clone(),
                )
                    .await
                    .unwrap());
            }
        };
    }
}
