use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use crate::clients::{TorrentClientsEnum};
use crate::clients::flood::Flood;
use crate::clients::rtorrent::rTorrent;
use crate::command_processor::commands::CommandProcessor;
use crate::config::config::{Config, SecurityMode, TorrentClientOption, TorrentPlatforms};
use crate::irc_processor::irc::IrcProcessor;
use crate::platforms::{TorrentPlatform, TorrentPlatformsEnum};
use crate::platforms::tl::TorrentLeech;
use crate::torrent_processor::torrent::TorrentProcessor;
use tokio::select;
use tokio::time::{Duration, Instant, interval_at};

mod irc_processor;
mod command_processor;
mod torrent_processor;
mod config;
mod clients;
mod platforms;
mod auth;

static IRC_CONFIG_FILE: &str = "irc.toml";
static OPTIONS_CONFIG_FILE: &str = "options.toml";
const PERIODIC_CHECK_INTERVAL: u64 = 60;

async fn periodic_check(irc: Rc<RefCell<IrcProcessor>>, nick: &str) {
    let start_time = Instant::now();
    let mut interval = interval_at(start_time, Duration::from_secs(PERIODIC_CHECK_INTERVAL));
    loop {
        irc.borrow().update_user_status(nick);
        interval.tick().await;
    }
}

pub struct Irc2Torrent {
    config: Rc<RefCell<Config>>,
    torrent_processor: Rc<TorrentProcessor>,
    command_processor: Box<Rc<CommandProcessor>>,
    irc_processor: Rc<RefCell<IrcProcessor>>,
}
const CLIENT_MAX_RETRY: u8 = 10;
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
        let config = Rc::new(RefCell::new(cfg));
        // let re: Regex = Regex::new(r".*Name:'(?P<name>.*)' uploaded by.*https://www.torrentleech.org/torrent/(?P<id>\d+)").unwrap();
        let torrent_processor = Rc::new(
            TorrentProcessor::new(config.clone(), torrent_ch, vec![commands.clone().subscribe(), irc.clone().subscribe()], torrent_client, torrent_platform));
        let command_processor = Rc::new(
            CommandProcessor::new(config.clone(), torrent_processor.clone(), command_ch, vec![torrent.clone().subscribe(), irc.clone().subscribe()]));
        let irc_processor = Rc::new(RefCell::new(
            IrcProcessor::new(config.clone(), torrent_processor.clone(), command_processor.clone(), irc_ch, vec![torrent.clone().subscribe(), commands.clone().subscribe()])));
        /*if let SecurityMode::IrcUserName(nick) = config.borrow().get_security_mode() {
            select! {
                _ = periodic_check(irc_processor.clone(), &nick) => {}
            }
        }*/
        Self { config, torrent_processor, command_processor: Box::new(command_processor), irc_processor: irc_processor }
    }

    pub async fn start(&mut self) {
        self.irc_processor.borrow_mut().start_listening().await;
    }

    async fn get_torrent_client(clients: &mut TorrentClientOption) -> TorrentClientsEnum {
        let mut retry_count: u8 = 0;
        loop {
            if retry_count >= CLIENT_MAX_RETRY {
                panic!("Failed to connect to torrent client after {} retries", retry_count);
            }
            retry_count += 1;
            if retry_count > 1 {
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            }
            let client = match clients {
                TorrentClientOption::rTorrent(ref mut c) => {
                    rTorrent::new(c.xmlrpc_url.clone()).await.map(TorrentClientsEnum::Rtorrent)
                }
                TorrentClientOption::Flood(ref mut c) => {
                    Flood::new(
                        c.username.clone(),
                        c.password.clone(),
                        c.url.clone(),
                        c.destination.clone(),
                    )
                        .await
                        .map(TorrentClientsEnum::Flood)
                }
            };
            if let Ok(c) = client {
                break c;
            }
        }
    }
}
