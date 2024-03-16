pub mod irc {
    use std::any::Any;
    use std::cell::RefCell;
    use std::collections::HashMap;
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
    use crate::auth;
    use crate::auth::Authorization;
    use crate::auth::AuthResult::*;
    use crate::auth::MessageTypes::{Announcement};

    use crate::command_processor::commands::CommandProcessor;
    use crate::config::config::SecurityMode;
    use crate::platforms::TorrentPlatform;
    use crate::torrent_processor::torrent::TorrentProcessor;

    const IRC_MAX_RETRY: u8 = 10;

    pub struct IrcProcessor {
        evt_channel: PubSub<String>,
        subs_cfg: Vec<Subscription<String>>,
        config: Rc<RefCell<crate::config::config::Config>>,
        tp: Rc<TorrentProcessor>,
        cp: Rc<CommandProcessor>,
        client: Rc<RefCell<Option<Client>>>,
        status_response_regex: Regex,
        auth: Authorization,
        user_status: HashMap<String, UserStatus>,
    }
    
    #[derive(Debug)]
    pub struct UserStatus {
        nick: String,
        status: u8,
        time_of_check: u64,
    }

    impl IrcProcessor {
        pub fn new(cfg: Rc<RefCell<crate::config::config::Config>>, torrent_processor: Rc<TorrentProcessor>, command_processor: Rc<CommandProcessor>, evt_channel: PubSub<String>, subs_cfg: Vec<Subscription<String>>) -> Self {
            Self { config: cfg.clone(), tp: torrent_processor, cp: command_processor, evt_channel, subs_cfg, client: Rc::new(RefCell::new(None)), status_response_regex: Regex::new(r"STATUS (?P<nick>\w+) (?P<status>\d{1})").unwrap(), auth: Authorization::new(cfg.clone()), user_status: HashMap::new() }
        }

        pub async fn start_listening(&mut self) {
            let mut retry_count = 0;
            'connection: loop {
                if let Some(mut ok_stream) = self.connect_irc().await {
                    'message: loop {
                        match ok_stream.next().await.transpose() {
                            Ok(Some(msg)) => {
                                self.msg_process(&msg).await;
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
                        break 'connection;
                    }
                }
            }
        }

        async fn msg_process(&mut self, msg: &Message) {
            if let (Command::PRIVMSG(channel, inner_message), Some(nick)) = (&msg.command, &msg.source_nickname()) {
                info!("{}@{}: {}", nick, channel, inner_message);
                let re = self.config.borrow().get_announce_regex().clone();
                if let Some(caps) = re.captures(inner_message) {
                    let (name, id) = (&caps["name"], &caps["id"]);
                    self.torrent_msg_process(channel, nick, &name, &id).await;
                } else {
                    if self.cp.is_command(inner_message) {
                        self.command_msg_process(channel, inner_message, nick).await;
                    } else if channel.eq("NickServ") {
                        info!("Message is from NickServ.");
                        if inner_message.contains("STATUS") {
                            info!("Message is a STATUS response.");
                            let (nick, status) = self.status_response_regex.captures(inner_message).map(|caps| (caps["nick"].to_string(), caps["status"].parse().unwrap())).unwrap();
                            self.user_status_report(nick.as_str(), status);
                        }
                    } else {
                        info!("Message is not a torrent or a command. ({nick}: {inner_message})");
                    }
                }
            }
        }

        async fn command_msg_process(&mut self, channel: &String, inner_message: &String, nick: &&str) {
            info!("Message is a command. ({nick}: {inner_message})");
            match self.auth.authenticate(nick, channel, inner_message, crate::auth::MessageTypes::Command) {
                NotAuthorized => {
                    error!("User is not authorized to use this bot.");
                    let _ = self.send_privmsg(channel, "You are not authorized to use this bot.");
                }
                _ => {
                    if let Ok(result) = self.cp.process_command(inner_message.to_string()).await {
                        info!("Command result: {}", result);
                        let _ = self.send_privmsg(channel, result.as_str());
                    } else {
                        error!("Command failed.");
                        let _ = self.send_privmsg(channel, "Command not found.");
                    }
                }
            }
        }

        async fn torrent_msg_process(&mut self, channel: &String, nick: &&str, name: &&str, id: &&str) {
            let inner_message: &String;

            info!("Torrent name: {}", name);
            info!("Torrent Id: {}", id);
            if let SourceValidated = self.auth.authenticate(nick, channel, "", Announcement) {
                info!("User is authenticated.");
                if self.tp.process_torrent(&name.to_string(), &id.to_string()).await {
                    let _ = self.send_privmsg(channel, "Torrent added to client.");
                } else {
                    let _ = self.send_privmsg(channel, "Could not add torrent to client.");
                }
            }
        }

        pub fn user_status_report(&mut self, nick: &str, status: u8) {
            let time = chrono::Utc::now().timestamp();
            let user = UserStatus { nick: nick.to_string(), status, time_of_check: time as u64 };
            info!("User status report: {user:?}");
            self.user_status.insert(nick.to_string(), user);
        }
        
        pub fn update_user_status(&self, nick: &str) {
            if let Some(c) = self.client.borrow_mut().as_mut() {
                let _ = c.send_privmsg("NickServ", format!("STATUS {}", nick));
            }
        }
        
        pub fn send_log(&self, message: &str) {
            if let Some(c) = self.client.borrow_mut().as_mut() {
                let _ = c.send_privmsg("NickServ", message);
            }
        }

        fn send_privmsg(&self, channel: &str, message: &str) {
            if let Some(c) = self.client.borrow_mut().as_mut() {
                let _ = c.send_privmsg(channel, message);
            }
        }

        pub async fn connect_irc(&mut self) -> Option<ClientStream> {
            let cli: Option<ClientStream> = match Client::from_config(self.config.borrow().get_irc_config()).await {
                Ok(mut c) => {
                    if let Ok(_) = c.identify() {
                        if let Ok(cs) = c.stream() {
                            self.client = Rc::new(RefCell::new(Some(c)));
                            info!("Connected to IRC server.");
                            println!("Connected to IRC server.");
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
                    println!("Could not connect to IRC server.{e:?}");
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