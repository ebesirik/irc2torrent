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
    use crate::config::config::SecurityMode;
    use crate::platforms::TorrentPlatform;
    use crate::torrent_processor::torrent::TorrentProcessor;
    
    const IRC_MAX_RETRY: u8 = 10;

    pub struct IrcProcessor {
        evt_channel: PubSub<String>,
        subs_cfg: Vec<Subscription<String>>,
        config: Arc<Mutex<crate::config::config::Config>>,
        tp: Arc<Mutex<TorrentProcessor>>,
        cp: Arc<Mutex<CommandProcessor>>,
        client: Arc<Mutex<Option<Client>>>,
    }

    impl IrcProcessor {
        pub fn new(cfg: Arc<Mutex<crate::config::config::Config>>, torrent_processor: Arc<Mutex<TorrentProcessor>>, command_processor: Arc<Mutex<CommandProcessor>>, evt_channel: PubSub<String>, subs_cfg: Vec<Subscription<String>>) -> Self {
            Self { config: cfg, tp: torrent_processor, cp: command_processor, evt_channel, subs_cfg, client: Arc::new(Mutex::new(None)) }
        }

        pub async fn start_listening(&mut self) {
            let mut retry_count = 0;
            loop {
                if let Some(mut ok_stream) = self.connect_irc().await {
                    loop {
                        match ok_stream.next().await.transpose() {
                            Ok(Some(msg)) => {
                                if let (Command::PRIVMSG(channel, inner_message), Some(nick)) = (&msg.command, &msg.source_nickname()) {
                                    info!("{}@{}: {}", nick, channel, inner_message);
                                    if let Some(caps) = self.config.lock().unwrap().get_announce_regex().captures(inner_message) {
                                        let (name, id) = (&caps["name"], &caps["id"]);
                                        info!("Torrent name: {}", name);
                                        info!("Torrent Id: {}", id);
                                        if self.tp.lock().unwrap().do_we_want_this_torrent(&name.to_string()) {
                                            if let Ok(b64) = self.tp.lock().unwrap().download_torrent(name.to_string(), id.to_string()).await {
                                                self.tp.lock().unwrap().add_torrent_and_start(b64, name.to_string()).await;
                                            }
                                            break;
                                        }
                                    } else {
                                        if self.cp.lock().unwrap().is_command(inner_message) {
                                            if self.config.lock().unwrap().is_commands_enabled() {
                                                if self.cp.lock().unwrap().authenticate(nick, &inner_message) {
                                                    if let Ok(result) = self.cp.lock().unwrap().process_command(inner_message.to_string()).await {
                                                        let _ = self.client.clone().lock().unwrap().as_ref().unwrap().send_privmsg(channel, result);
                                                    } else {
                                                        let _ = self.client.clone().lock().unwrap().as_ref().unwrap().send_privmsg(channel, "Command not found.");
                                                    }
                                                } else {
                                                    let _ = self.client.clone().lock().unwrap().as_ref().unwrap().send_privmsg(channel, "You are not authorized to use this bot.");
                                                }
                                            } else {
                                                let _ = self.client.clone().lock().unwrap().as_ref().unwrap().send_privmsg(channel, "Commands are disabled.");
                                            }
                                        } else {
                                            info!("Message is not a torrent or a command. ({nick}: {inner_message})");
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                if e.type_id() == Error::PingTimeout.type_id() {
                                    if let Some(stream) = self.connect_irc().await {
                                        info!("Reconnected to IRC server.");
                                        ok_stream = stream;
                                    } else {
                                        error!("Could not reconnect to IRC server.");
                                        let _ = task::sleep(time::Duration::from_secs(3));
                                    }
                                } else {
                                    error!("{:?}", e);
                                }
                            }
                        }
                    }
                } else {
                    retry_count += 1;
                    error!("Could not reconnect to IRC server. Waiting 3 seconds to try again");
                    let _ = task::sleep(time::Duration::from_secs(3));
                    if retry_count >= IRC_MAX_RETRY {
                        error!("Could not reconnect to IRC server after {} retries. Exiting.", IRC_MAX_RETRY);
                        break;
                    }
                }
            }
        }

        pub async fn connect_irc(&mut self) -> Option<ClientStream> {
            let cli: Option<ClientStream> = match Client::from_config(self.config.lock().unwrap().get_irc_config().clone()).await {
                Ok(mut c) => {
                    if let Ok(_) = c.identify() {
                        if let Ok(cs) = c.stream() {
                            self.client = Arc::new(Mutex::new(Some(c)));
                            info!("Connected to IRC server.");
                            Some(cs)
                        } else {
                            error!("Could not get client stream.");
                            None
                        }
                    } else {
                        error!("Could not identify with server.");
                        None
                    }
                }
                Err(e) => {
                    error!("Could not connect to IRC server.{e:?}");
                    None
                }
            };
            cli
        }
    }
}
