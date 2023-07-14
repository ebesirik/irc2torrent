use regex::Regex;
use crate::Config;
use crate::torrent_processor::TorrentProcessor;

pub struct CommandProcessor {
    pub config: Config,
    pub tp: TorrentProcessor,
    pub release_catching_regex: Regex,
    pub command_catching_regex: Regex
}
impl CommandProcessor {
//generate functions for CRUD operations on borrowed options from suplied message string as parameter if string is a valid command
    //return true if command was found and executed, false otherwise
    pub async fn process_command(&self, message: String) -> bool {
        if let Some(caps) = self.command_catching_regex.captures(message.as_str()) {
            let (command, argument) = (&caps["command"] as &str, &caps["argument"] as &str);
            info!("Command: {}", command);
            info!("Argument: {}", argument);
            match command {
                "add" => {
                    if self.tp.do_we_want_this_torrent(&argument.to_string()) {
                        if let Ok(b64) = self.tp.download_torrent(argument.to_string(), "0".to_string()).await {
                            self.tp.add_torrent_and_start(b64, argument.to_string()).await;
                        }
                        return true;
                    }
                },
                "remove" => {
                    return true;
                },
                "list" => {
                    return true;
                },
                _ => {
                    return false;
                }
            }
        }
        false
    }

}
