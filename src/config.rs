pub mod config {
    use std::path::PathBuf;

    use anyhow::Error;
    use directories::BaseDirs;
    use log::{error, info};
    use regex::Regex;
    use serde::{de, ser};
    use serde_derive::{Deserialize, Serialize};
    use tokio::{fs, io};

    use crate::{IRC_CONFIG_FILE, OPTIONS_CONFIG_FILE};

    pub struct Config {
        option_data: OptionData,
        irc_data: irc::client::data::config::Config,
    }
    
    impl Default for Config {
        fn default() -> Self {
            Self { option_data: OptionData::default(), irc_data: Config::get_irc_default_config() }
        }
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct OptionData {
        platform: TorrentPlatforms,
        clients: Vec<TorrentClientOption>,
        command_options: CommandOptions,
        regex_for_downloads_match: Vec<String>,
        regex_for_announce_match: String,
    }
    
    impl Default for OptionData {
        fn default() -> Self {
            Self { 
                platform: TorrentPlatforms::TorrentLeech(TorrentLeechOptions::default()), 
                clients: vec![TorrentClientOption::rTorrent(rTorrentOptions::default()),
                              TorrentClientOption::Flood(FloodOptions::default())], 
                command_options: CommandOptions::default(),
                regex_for_downloads_match: vec!["Some Regex to match.*1080p.*".to_string(), "Another Release.*S02.*1080p.*WEB.*".to_string()],
                regex_for_announce_match: r".*Name:'(?P<name>.*)' uploaded by.*https://www.torrentleech.org/torrent/(?P<id>\d+)".to_string()
            }
        }
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct CommandOptions {
        security_mode: SecurityMode,
        commands_enabled: bool,
    }
    
    impl Default for CommandOptions {
        fn default() -> Self {
            Self { security_mode: SecurityMode::IrcUserName("irc2torrent".to_string()), commands_enabled: false }
        }
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub enum SecurityMode {
        Password(String),
        IrcUserName(String),
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub enum TorrentClientOption {
        rTorrent(rTorrentOptions),
        Flood(FloodOptions),
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct rTorrentOptions {
        pub xmlrpc_url: String,
    }
    
    impl Default for rTorrentOptions {
        fn default() -> Self {
            Self { xmlrpc_url: "unix://config/.local/share/rtorrent/rtorrent.sock".to_string() }
        }
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct FloodOptions {
        pub(crate) url: String,
        pub(crate) username: String,
        pub(crate) password: String,
        pub(crate) destination: String,
    }

    impl Default for FloodOptions {
        fn default() -> Self {
            Self { url: "http://localhost:3000".to_string(), username: "admin".to_string(), password: "password".to_string(), destination: "/downloads".to_string() }
        }
    }
        
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub enum TorrentPlatforms {
        TorrentLeech(TorrentLeechOptions),
    }
    
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct TorrentLeechOptions {
        pub(crate) rss_key: String,
        pub(crate) torrent_dir: String,
    }
    
    impl Default for TorrentLeechOptions {
        fn default() -> Self {
            Self { rss_key: "XXXXXXXX".to_string(), torrent_dir: "/tmp".to_string() }
        }
    }

    impl Config {
        pub async fn new() -> Result<Config, Error> {
            return if let (Some(option_config), Some(irc_config)) = (Config::read_or_create_toml::<OptionData>(OPTIONS_CONFIG_FILE.to_string(), Some(&OptionData::default())).await,
                                                                     Config::read_or_create_toml::<irc::client::data::config::Config>(IRC_CONFIG_FILE.to_string(), Some(&Self::get_irc_default_config())).await) {
                Ok(Self { option_data: option_config, irc_data: irc_config/*, subscribers: Mutex::new(HashSet::new())*/ })
            } else {
                Err(Error::msg("Could not read or create options file"))
            }
        }
        
        fn get_irc_default_config() -> irc::client::data::config::Config {
            return irc::client::data::config::Config {
                nickname: Some("irc2torrent".to_string()),
                nick_password: Some("password".to_string()),
                alt_nicks: vec!["irc2torrent_".to_string(), "irc2torrent__".to_string(), "irc2torrent___".to_string(), "irc2torrent____".to_string(), "irc2torrent_____".to_string(), "irc2torrent______".to_string()],
                username: Some("irc2torrent".to_string()),
                realname: Some("irc2torrent".to_string()),
                server: Some("irc.torrentleech.org".to_string()),
                port: Some(7011),
                channels: vec!["#tlannounces".to_string()],
                user_info: Some("I'm a bot user for the irc2torrent daemon.".to_string()),
                source: Some("https://github.com/ebesirik/irc2torrent".to_string()),
                ..irc::client::data::config::Config::default()
            };
        }
        
        pub fn is_commands_enabled(&self) -> bool {
            return self.option_data.command_options.commands_enabled;
        }
        
        pub fn get_security_mode(&self) -> &SecurityMode {
            return &self.option_data.command_options.security_mode;
        }
        
        pub fn get_torrent_client(&mut self) -> TorrentClientOption {
            return self.option_data.clients.first_mut().unwrap().clone();
        }
        
        pub fn get_torrent_platform(&self) -> &TorrentPlatforms {
            return &self.option_data.platform;
        }
        
        pub fn get_announce_regex(&self) -> Regex {
            return Regex::new(&self.option_data.regex_for_announce_match.as_str()).unwrap();
        }

        pub fn get_dl_regexes(&self) -> Vec<Regex> {
            return self.option_data.regex_for_downloads_match.iter().filter_map(|regex| Regex::new(regex.as_str()).ok()).collect();
        }

        pub async fn add_dl_regex(&mut self, regex: String) {
            self.option_data.regex_for_downloads_match.push(regex);
            let _ = self.update_option_file(OPTIONS_CONFIG_FILE.to_string(), &self.option_data).await;
        }

        pub async fn remove_dl_regex(&mut self, regex: usize) {
            self.option_data.regex_for_downloads_match.remove(regex);
            let _ = self.update_option_file(OPTIONS_CONFIG_FILE.to_string(), &self.option_data).await;
        }

        pub fn get_irc_config(&self) -> irc::client::data::Config {
            return self.irc_data.clone();
        }

        async fn read_or_create_toml<T>(filename: String, data: Option<&T>) -> Option<T>
            where T: ser::Serialize, T: de::DeserializeOwned {
            if let Some(full_path_buf) = Config::get_full_config_path(filename.clone()) {
                info!("You can edit the config file at '{}' location", full_path_buf.to_str()?);
                return if full_path_buf.exists() {
                    let path = full_path_buf.as_path();
                    let contents: String = match fs::read_to_string(path).await {
                        Ok(c) => c,
                        Err(_) => {
                            error!("Could not read file `{}`", path.to_str()?);
                            return None;
                        }
                    };
                    match toml::from_str(&contents) {
                        Ok(d) => d,
                        Err(_) => {
                            error!("Unable to load data from `{}`", path.to_str()?);
                            return None;
                        }
                    }
                } else {
                    if let Some(result) = data {
                        let toml = toml::to_string(result).unwrap();
                        let path = full_path_buf.as_path();
                        match fs::write(path, toml).await {
                            Ok(_) => info!("New options file created at '{}' location, please consider modifying it before running to app.", path.to_str()?),
                            Err(_) => error!("Error creating {} file", path.to_str()?)
                        };
                    }
                    None
                }
            }

            return None;
        }

        fn get_full_config_path(filename: String) -> Option<PathBuf> {
            if let Some(proj_dir) = BaseDirs::new() {
                let dir = proj_dir.config_dir();
                let full_path_buf = dir.join(filename);
                return Some(full_path_buf);
            }
            return None;
        }

        pub async fn update_option_file<T>(&self, filename: String, config: T) -> Result<bool, String>
            where T: ser::Serialize {
            if let Ok(toml) = toml::to_string(&config) {
                if let Some(path) = Config::get_full_config_path(filename) {
                    return match fs::write(path, toml).await {
                        Ok(_) => {
                            info!("Options file updated");
                            Ok(true)
                        }
                        _ => {
                            error!("Error updating options file");
                            Err("Could not update options file".to_string())
                        }
                    };
                };
            } else {
                error!("Error updating options file");
                return Err("Could not update options file".to_string());
            }
            return Err("Could not update options file".to_string());
        }
    }
}
