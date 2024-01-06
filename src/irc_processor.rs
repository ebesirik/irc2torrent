pub mod irc {
    use std::any::Any;
    use std::sync::{Arc, Mutex};
    use std::time;

    use async_std::task;
    use futures::prelude::*;
    use irc::client::ClientStream;
    use irc::client::prelude::*;
    use irc::error::Error;
    use irc::proto::Command;
    use log::{error, info};
    use pub_sub::{PubSub, Subscription};
    use regex::Regex;

    use crate::command_processor::commands::CommandProcessor;
    use crate::torrent_processor::torrent::TorrentProcessor;

    pub struct IrcProcessor {
        evt_channel: PubSub<String>,
        subs_cfg: Vec<Subscription<String>>,
        config: Arc<Mutex<crate::config::config::Config>>,
        release_catching_regex: Regex,
        tp: Arc<Mutex<TorrentProcessor>>,
        cp: Arc<Mutex<CommandProcessor>>,
    }

    impl IrcProcessor {
        pub fn new(cfg: Arc<Mutex<crate::config::config::Config>>, rcr: Regex, torrent_processor: Arc<Mutex<TorrentProcessor>>, command_processor: Arc<Mutex<CommandProcessor>>, evt_channel: PubSub<String>, subs_cfg: Vec<Subscription<String>>) -> Self {
            Self { config: cfg, release_catching_regex: rcr, tp: torrent_processor, cp: command_processor, evt_channel, subs_cfg }
        }

        pub async fn start_listening(&self) {
            if let Some(mut ok_stream) = self.connect_irc().await {
                loop {
                    match ok_stream.next().await.transpose() {
                        Ok(msg) => {
                            if let Some(message) = msg {
                                if let (Command::PRIVMSG(channel, inner_message), Some(nick)) = (&message.command, &message.source_nickname()) {
                                    println!("channel: {:?}", channel);
                                    println!("message: {:?}", inner_message);
                                    println!("source nick: {:?}", nick);
                                    info!("{}@{}: {}", nick, channel, inner_message);
                                    if let Some(caps) = self.release_catching_regex.captures(inner_message) {
                                        let (name, id) = (&caps["name"], &caps["id"]);
                                        info!("Torrent name: {}", name);
                                        info!("Torrent Id: {}", id);
                                        if self.tp.lock().unwrap().do_we_want_this_torrent(&name.to_string()) {
                                            if let Ok(b64) = self.tp.lock().unwrap().download_torrent(name.to_string(), id.to_string()).await {
                                                self.tp.lock().unwrap().add_torrent_and_start(b64, name.to_string()).await;
                                            }
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            if e.type_id() == Error::PingTimeout.type_id() {
                                if let Some(stream) = self.connect_irc().await {
                                    ok_stream = stream;
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

        pub async fn connect_irc(&self) -> Option<ClientStream> {
            let cli: Option<ClientStream> = match Client::from_config(self.config.lock().unwrap().get_irc_config().clone()).await {
                Ok(mut c) => {
                    if let Ok(_) = c.identify() {
                        if let Ok(cs) = c.stream() {
                            Some(cs)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
                Err(_) => None
            };
            cli
        }
    }
}
