use regex::Regex;
use toml::Value;
use crate::Config;
use crate::torrent_processor::TorrentProcessor;

pub struct CommandProcessor<'cp, 'tp> {
    config: &'cp Config,
    tp: &'cp mut TorrentProcessor<'tp>,
    command_catching_regex: Regex,
    announce_regex: Regex,
}

impl<'cp, 'tp> CommandProcessor<'cp, 'tp> {
    pub fn new(cfg: &'cp Config, torrent_processor: &'tp mut TorrentProcessor, announce_regex: Regex) -> Self {
        Self {
            config: cfg,
            command_catching_regex: Regex::new("(?P<command>[a-z]):(?P<params>.*)").unwrap(),
            tp: torrent_processor,
            announce_regex: announce_regex,
        }
    }
    //generate functions for CRUD operations on borrowed options from supplied message string as parameter if string is a valid command
    //return true if command was found and executed, false otherwise
    pub async fn process_command(&self, message: String) -> Result<String, String> {
        if let Some(caps) = self.command_catching_regex.captures(message.as_str()) {
            let (command, argument) = (&caps["command"], &caps["params"]);
            let args: Value = serde_json::from_str(argument).map_err(|_| Value::Array(vec![])).unwrap();
            info!("Command: {}", command);
            info!("Argument: {}", argument);
            match command {
                "addtorrent" => {
                    return self.process_result(self.add_torrent(argument).await);
                }
                "addtowatchlist" => {
                    return self.process_result(self.add_torrent_to_watchlist(argument));
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

    async fn remove_watch(&mut self, idx: usize) -> Result<String, String> {
        let mut err_str = "Wrong argument format. Use: removewatch <torrent name> <torrent id>";
        return self.tp.remove_torrent_from_watchlist(idx);
        Err(err_str.to_string())
    }


    async fn add_torrent(&self, argument: &str) -> Result<String, String> {
        let mut err_str = "Wrong argument format. Use: addtorrent <torrent name> <torrent id>";
        if let Some(caps) = self.announce_regex.captures(argument) {
            return self.tp.add_torrent(&caps["name"], &caps["id"]).await;
        }
        Err(err_str.to_string())
    }

    fn add_torrent_to_watchlist(&self, argument: &str) -> Result<String, String> {
        return self.tp.add_torrent_to_watchlist(argument.to_string());
    }
}
