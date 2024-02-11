pub mod config {
    //use std::collections::HashSet;
    use serde_derive::{Deserialize, Serialize};
    use directories::BaseDirs;
    use regex::Regex;
    use tokio::{fs, io};
    use std::path::{PathBuf};
    //use std::rc::{Rc, Weak};
    use serde::{de, ser};
    //use std::sync::{Mutex};
    use log::{error, info};
    use crate::{IRC_CONFIG_FILE, OPTIONS_CONFIG_FILE};

    pub struct Config {
        option_data: OptionData,
        irc_data: irc::client::data::config::Config,
        defaults: Defaults,
    }

    #[derive(Serialize, Deserialize)]
    pub struct OptionData {
        rss_key: String,
        rtorrent_xmlrpc_url: String,
        regex_for_downloads_match: Vec<String>,
    }

    struct Defaults {
        pub irc_defaults: irc::client::data::config::Config,
        pub options_defaults: OptionData,
    }

    impl Config {
        pub async fn new() -> Result<Config, io::Error> {
            let default = Defaults {
                options_defaults: OptionData {
                    rss_key: "XXXXXXXXXXXXXX".to_string(),
                    rtorrent_xmlrpc_url: "unix://config/.local/share/rtorrent/rtorrent.sock".to_string(),
                    regex_for_downloads_match: [
                        "Some Regex to match.*1080p.*".to_string(),
                        "Another Release.*S02.*1080p.*WEB.*".to_string()
                    ].to_vec(),
                },
                irc_defaults: irc::client::data::config::Config {
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
                },
            };
            return if let (Some(option_config), Some(irc_config)) = (Config::read_or_create_toml::<OptionData>(OPTIONS_CONFIG_FILE.to_string(), Some(&default.options_defaults)).await,
                                                                     Config::read_or_create_toml::<irc::client::data::config::Config>(IRC_CONFIG_FILE.to_string(), Some(&default.irc_defaults)).await) {
                Ok(Self { option_data: option_config, irc_data: irc_config, defaults: default/*, subscribers: Mutex::new(HashSet::new())*/ })
            } else {
                Err(io::Error::new(io::ErrorKind::Other, "Could not read or create options file"))
            }
        }

        pub fn get_dl_regexes(&self) -> Vec<Regex> {
            /*let mut torrent_names_regexes: Vec<Regex> = self.option_data.regex_for_downloads_match.iter().filter_map(|regex| Regex::new(regex.as_str()).ok()).collect();
            let mut torrent_names_regexes: Vec<Regex> = Vec::new();
            for downloads_match in &self.option_data.regex_for_downloads_match {
                if let Ok(dr) = Regex::new(downloads_match.as_str()) {
                    torrent_names_regexes.push(dr);
                }
            }*/
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

        pub fn get_rss_key(&self) -> String {
            return self.option_data.rss_key.clone();
        }

        pub fn get_xmlrpc_url(&self) -> String {
            return self.option_data.rtorrent_xmlrpc_url.clone();
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
