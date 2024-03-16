pub mod irc {
    use std::any::Any;
    use std::rc::Rc;
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
        config: Rc<Mutex<crate::config::config::Config>>,
        tp: Rc<Mutex<TorrentProcessor>>,
        cp: Rc<Mutex<CommandProcessor>>,
        client: Rc<Mutex<Option<Client>>>,
    }

    impl IrcProcessor {
        pub fn new(cfg: Rc<Mutex<crate::config::config::Config>>, torrent_processor: Rc<Mutex<TorrentProcessor>>, command_processor: Rc<Mutex<CommandProcessor>>, evt_channel: PubSub<String>, subs_cfg: Vec<Subscription<String>>) -> Self {
            Self { config: cfg, tp: torrent_processor, cp: command_processor, evt_channel, subs_cfg, client: Rc::new(Mutex::new(None)) }
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
                                            info!("Message is a command. ({nick}: {inner_message})");
                                            if self.config.lock().unwrap().is_commands_enabled() {
                                                info!("Commands are enabled.");
                                                if self.cp.lock().unwrap().authenticate(nick, &inner_message) {
                                                    info!("User is authenticated.");
                                                    if let Ok(result) = self.cp.lock().unwrap().process_command(inner_message.to_string()).await {
                                                        info!("Command result: {}", result);
                                                        let _ = self.client.clone().lock().unwrap().as_ref().unwrap().send_privmsg(channel, result);
                                                    } else {
                                                        error!("Command failed.");
                                                        let _ = self.client.clone().lock().unwrap().as_ref().unwrap().send_privmsg(channel, "Command not found.");
                                                    }
                                                } else {
                                                    error!("User is not authenticated.");
                                                    let _ = self.client.clone().lock().unwrap().as_ref().unwrap().send_privmsg(channel, "You are not authorized to use this bot.");
                                                }
                                            } else {
                                                error!("Commands are disabled.");
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
                            _ => {
                                error!("Something unexpected came from IRC server.");
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
                            self.client = Rc::new(Mutex::new(Some(c)));
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

#[cfg(test)]
pub mod test {
    
    #[tokio::test]
    pub async fn test_regex() {
        let re: regex::Regex = regex::Regex::new(r".*Name:'(?P<name>.*)' uploaded by.*https://www.torrentleech.org/torrent/(?P<id>\d+)").unwrap();
        let caps = re.captures("New Torrent Announcement: <TV :: BoxSets>  Name:'Secrets of Sulphur Springs S01 1080p AMZN WEB-DL DDP5 1 H 264-TVSmash' uploaded by 'Anonymous' freeleech -  https://www.torrentleech.org/torrent/241240312").unwrap();
        assert_eq!(&caps["name"], "Secrets of Sulphur Springs S01 1080p AMZN WEB-DL DDP5 1 H 264-TVSmash");
        assert_eq!(&caps["id"], "241240312");
    }
}