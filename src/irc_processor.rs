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
use crate::get_irc_config;
use crate::torrent_processor::TorrentProcessor;

pub struct IrcProcessor {
    config: Config,
    release_catching_regex: Regex,
    tp: TorrentProcessor,
    client: Option<Client>,
    stream: Option<ClientStream>
}

impl IrcProcessor {
    pub fn new(cfg: Config, rcr: Regex, torrent_processor: TorrentProcessor) -> Self {
        Self {config: cfg, release_catching_regex: rcr, tp: torrent_processor, client: None, stream: None}
    }

    pub async fn start_listening(&mut self){
        if let Ok(_) = self.connect_irc().await {
            if let Ok(str) = self.client.as_mut().unwrap().stream(){
                self.stream = Some(str);
                loop {
                    // let result =
                    match self.stream.as_mut().unwrap().next().await.transpose() {
                        Ok(msg) => {
                            if let Some(message) = msg {
                                if let (Command::PRIVMSG(channel, inner_message), Some(nick)) = (&message.command, &message.source_nickname()) {
                                    println!("channel: {:?}", channel);
                                    println!("message: {:?}", inner_message);
                                    println!("source nick: {:?}", nick);
                                    /*}
                                let msg_str = message.to_string();*/
                                    info!("{}@{}: {}", nick, channel, inner_message);
                                    if let Some(caps) = self.release_catching_regex.captures(inner_message) {
                                        let (name, id) = (&caps["name"] as &str, &caps["id"] as &str);
                                        info!("Torrent name: {}", name);
                                        info!("Torrent Id: {}", id);
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
                                if let Ok(clnt) = Client::from_config(get_irc_config()).await{
                                    self.client = Some(clnt);
                                    self.client.as_mut().unwrap().identify().unwrap();
                                    self.stream = Some(self.client.as_mut().unwrap().stream().unwrap());
                                } else {
                                    let _ = task::sleep(time::Duration::from_secs(1));
                                }
                            } else {
                                error!("{:?}", e);
                            }
                        }
                    }
                }
            }
        }
    }

    pub async fn connect_irc(&mut self) -> Result<(), ()>{
        if let Ok(cli) = Client::from_config(self.config.clone()).await{
            self.client = Some(cli);
            let _ = self.client.as_mut().unwrap().identify();
            return Ok(());
        } else {
            return Err(());
        }
    }
}