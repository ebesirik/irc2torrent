use std::any::Any;
use std::time;
use async_std::task;
use irc::client::ClientStream;
use irc::client::prelude::*;
use futures::prelude::*;
use irc::client::data::Config;
use irc::error::Error;
use irc::proto::Command;
use regex::Regex;
use crate::torrent_processor::TorrentProcessor;

pub struct IrcProcessor {
    config: Config,
    release_catching_regex: Regex,
    tp: TorrentProcessor,
}

impl<'a> IrcProcessor {
    pub fn new(cfg: Config, rcr: Regex, torrent_processor: TorrentProcessor) -> Self {
        Self {config: cfg, release_catching_regex: rcr, tp: torrent_processor}
    }

    pub async fn start_listening(&self){
        if let Some(mut ok_stream) = self.connect_irc().await {
            loop {
                match ok_stream.next().await.transpose() {
                    Ok(msg) => {
                        if let Some(message) = msg {
                            if let (Command::PRIVMSG(channel, inner_message), Some(nick)) = (&message.command, &message.source_nickname()) {
                                println!("channel: {:?}", channel);
                                println!("message: {:?}", inner_message);
                                println!("source nick: {:?}", nick);
                                println!("{}@{}: {}", nick, channel, inner_message);
                                if let Some(caps) = self.release_catching_regex.captures(inner_message) {
                                    let (name, id) = (&caps["name"] as &str, &caps["id"] as &str);
                                    println!("Torrent name: {}", name);
                                    println!("Torrent Id: {}", id);
                                    if self.tp.do_we_want_this_torrent(&name.to_string()) {
                                        if let Ok(b64) = self.tp.download_torrent(name.to_string(), id.to_string()).await {
                                            self.tp.add_torrent_and_start(b64, name.to_string()).await;
                                        }
                                        break;
                                    }
                                }
                            }
                        }
                    },
                    Err(e) => {
                        if e.type_id() == Error::PingTimeout.type_id() {
                            if let Some(stream) = self.connect_irc().await{
                                ok_stream = stream;
                            } else {
                                let _ = task::sleep(time::Duration::from_secs(1));
                            }
                        } else {
                            println!("{:?}", e);
                        }
                    }
                }
            }
        }
    }

    pub async fn connect_irc<'b: 'a>(&self) -> Option<ClientStream>{
        let cli:Option<ClientStream> = match Client::from_config(self.config.clone()).await {
            Ok(mut c) => {
                if let Ok(_) = c.identify(){
                    if let Ok(cs) = c.stream() {
                        Some(cs)
                    } else {
                        None
                    }
                } else {
                    None
                }
            },
            Err(_) => None
        };
        cli
    }
}