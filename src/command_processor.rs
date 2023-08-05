use regex::Regex;
use crate::Config;
use crate::torrent_processor::TorrentProcessor;

pub struct CommandProcessor<'cp, 'tp> {
    config: &'cp Config,
    tp: &'cp TorrentProcessor<'tp>,
    command_catching_regex: Regex,
    announce_regex: Regex,
}

impl<'cp, 'tp> CommandProcessor<'cp, 'tp> {
    pub fn new(cfg: &'cp Config, torrent_processor: &'tp TorrentProcessor, announce_regex: Regex) -> Self {
        Self { config: cfg, command_catching_regex: Regex::new("(?P<command>[a-z]):(?P<params>.*)").unwrap(), tp: torrent_processor, announce_regex: announce_regex}
    }
    //generate functions for CRUD operations on borrowed options from supplied message string as parameter if string is a valid command
    //return true if command was found and executed, false otherwise
    pub async fn process_command(&self, message: String) -> bool {
        if let Some(caps) = self.command_catching_regex.captures(message.as_str()) {
            let (command, argument) = (&caps["command"], &caps["params"]);
            info!("Command: {}", command);
            info!("Argument: {}", argument);
            match command {
                "addtorrent" => {
                    if let Some(value) = self.add_torrent(argument).await {
                        return value;
                    }
                    return false;
                }
                "addtowatchlist" => {
                    return true;
                }
                "removetorrent" => {
                    return true;
                }
                "removewatch" => {
                    return true;
                }
                "torrentlist" => {
                    return true;
                }
                "watchlist" => {
                    return true;
                }
                _ => {
                    return false;
                }
            }
        }
        false
    }

    async fn add_torrent(&self, argument: &str) -> Option<bool> {
        if let Ok(b64) = self.tp.download_torrent(argument.to_string(), "0".to_string()).await {
            self.tp.add_torrent_and_start(b64, argument.to_string()).await;
            return Some(true);
        }
        None
    }

    async fn add_torrent_to_watchlist(&self, argument: &str) -> bool {
        return true;
    }
}
