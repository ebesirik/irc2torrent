
pub mod commands {
    // use std::borrow::Borrow;
    use std::sync::{Arc, Mutex};

    use log::{error, info};
    use pub_sub::{PubSub, Subscription};
    use regex::Regex;

    use crate::Config;
    use crate::torrent_processor::torrent::TorrentProcessor;

    pub struct CommandProcessor {
        evt_channel: PubSub<String>,
        subs_cfg: Vec<Subscription<String>>,
        config: Arc<Mutex<Config>>,
        tp: Arc<Mutex<TorrentProcessor>>,
        command_catching_regex: Regex,
        announce_regex: Regex,
    }

    impl CommandProcessor {
        pub fn new(cfg: Arc<Mutex<Config>>, torrent_processor: Arc<Mutex<TorrentProcessor>>, announce_regex: Regex, evt_channel: PubSub<String>, subs_cfg: Vec<Subscription<String>>) -> Self {
            Self {
                config: cfg,
                command_catching_regex: Regex::new("(?P<command>[a-z]):(?P<params>.*)").unwrap(),
                tp: torrent_processor,
                announce_regex,
                evt_channel,
                subs_cfg,
            }
        }
        //generate functions for CRUD operations on borrowed options from supplied message string as parameter if string is a valid command
        //return true if command was found and executed, false otherwise
        pub async fn process_command(&self, message: String) -> Result<String, String> {
            if let Some(caps) = self.command_catching_regex.captures(message.as_str()) {
                let (command, argument) = (&caps["command"], &caps["params"]);
                // let args: Value = serde_json::from_str(argument).map_err(|_| Value::Array(vec![])).unwrap();
                info!("Command: {}", command);
                info!("Argument: {}", argument);
                match command {
                    "addtorrent" => {
                        return self.process_result(self.add_torrent(argument).await);
                    }
                    "addtowatchlist" => {
                        return self.process_result(self.add_torrent_to_watchlist(argument).await);
                    }
                    "removeanddeletetorrent" => {
                        return Err("Not implemented yet".to_string());
                    }
                    "removetorrent" => {
                        return Err("Not implemented yet".to_string());
                    }
                    "stoptorrent" => {
                        return Err("Not implemented yet".to_string());
                    }
                    "removewatch" => {
                        return Err("Not implemented yet".to_string());
                    }
                    "torrentlist" => {
                        return Err("Not implemented yet".to_string());
                    }
                    "watchlist" => {
                        return Err("Not implemented yet".to_string());
                    }
                    _ => {
                        return Err("Not implemented yet".to_string());
                    }
                }
            }
            Err("Not implemented yet".to_string())
        }

        fn process_result(&self, result: Result<String, String>) -> Result<String, String> {
            match result {
                Ok(r) => {
                    info!("{}", r);
                    Ok(r)
                }
                Err(e) => {
                    error!("{}", e);
                    Err(e)
                }
            }
        }

        async fn remove_watch(&self, idx: usize) -> Result<String, String> {
            // let mut err_str = "Wrong argument format. Use: removewatch <torrent name> <torrent id>";
            let r = &self.tp.clone().lock().unwrap().remove_torrent_from_watchlist(idx);
            return match r {
                Ok(std) => Ok(std.to_string()),
                Err(error) => Err(error.to_string()),
            };
            // Err(err_str.to_string())
        }


        async fn add_torrent(&self, argument: &str) -> Result<String, String> {
            let err_str = "Wrong argument format. Use: addtorrent <torrent name> <torrent id>";
            if let Some(caps) = self.announce_regex.captures(argument) {
                return self.tp.clone().lock().unwrap().add_torrent(&caps["name"], &caps["id"]).await;
            }
            Err(err_str.to_string())
        }

        async fn add_torrent_to_watchlist(&self, argument: &str) -> Result<String, String> {
            // let tp = self.tp.clone();
            return self.tp.lock().unwrap().add_torrent_to_watchlist(argument.to_owned()).await;
        }
    }
}

