pub mod torrent {
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::{Arc, Mutex};

    use anyhow::Error;
    use base64;
    use base64::Engine as _;
    use log::{error, info};
    use pub_sub::{PubSub, Subscription};
    use regex::Regex;

    use crate::clients::{DownloadResult, TorrentClientsEnum};
    use crate::config::config::Config;
    use crate::platforms::TorrentPlatformsEnum::TorrentLeech;
    use crate::platforms::{TorrentPlatform, TorrentPlatformsEnum};

    pub struct TorrentProcessor {
        evt_channel: PubSub<String>,
        subs_cfg: Vec<Subscription<String>>,
        torrent_client: TorrentClientsEnum,
        torrent_platform: TorrentPlatformsEnum,
        options: Rc<RefCell<Config>>,
        // dl_regexes: Vec<Regex>,
    }

    impl TorrentProcessor {
        pub fn new(
            config: Rc<RefCell<Config>>,
            evt_channel: PubSub<String>,
            subs_cfg: Vec<Subscription<String>>,
            torrent_client: TorrentClientsEnum,
            torrent_platform: TorrentPlatformsEnum,
        ) -> TorrentProcessor {
            // let dl_regex = config.lock().unwrap().get_dl_regexes().clone();
            Self {
                evt_channel,
                subs_cfg,
                torrent_client,
                torrent_platform,
                options: config,
                // dl_regexes: dl_regex,
            }
        }

        pub async fn process_torrent(&self, name: &String, id: &String) -> bool {
            if self.do_we_want_this_torrent(&name.to_string()) {
                if let Ok(b64) = self
                    .download_torrent(name.to_string(), id.to_string())
                    .await
                {
                    info!("Torrent downloaded.");
                    match self.add_torrent_and_start(b64, name.to_string()).await {
                        Ok(_) => {
                            info!("Torrent added to client.");
                            // let _ = self.send_privmsg(nick, channel, "Torrent added to client.");
                            return true;
                        }
                        Err(e) => {
                            error!("Could not add torrent to client. {:?}", e);
                            // let _ = self.send_privmsg(nick, channel, format!("Could not add torrent to client. Error was: {:?}", e).as_str());
                        }
                    }
                }
            }
            return false;
        }

        pub fn do_we_want_this_torrent(&self, name: &String) -> bool {
            let torrent_match_regex_list = self.options.borrow().get_dl_regexes();
            let torrent_match_reject_regex_list = self.options.borrow().get_reject_regexes();
            for regex in &torrent_match_reject_regex_list {
                if regex.is_match(name) {
                    info!("Torrent {} rejected by reject list", name);
                    return false;
                }
            }
            for regex in &torrent_match_regex_list {
                if regex.is_match(name) {
                    return true;
                }
            }
            return false;
        }

        pub async fn get_download_list(&mut self) -> Result<Vec<DownloadResult>, Error> {
            let mut list = match &self.torrent_client {
                TorrentClientsEnum::Rtorrent(c) => c.get_dl_list().await?,
                TorrentClientsEnum::Flood(c) => c.get_dl_list().await?,
            };
            Ok(list.to_owned())
        }

        pub async fn add_torrent_and_start(&self, file: String, name: String) -> Result<(), Error> {
            match &self.torrent_client {
                TorrentClientsEnum::Rtorrent(c) => c.add_torrent_and_start(&file, name).await,
                TorrentClientsEnum::Flood(c) => c.add_torrent_and_start(&file, name).await,
            }
        }

        pub async fn download_torrent(&self, name: String, id: String) -> Result<String, Error> {
            if let TorrentLeech(tl) = &self.torrent_platform {
                return tl.download_torrent(name, id).await;
            } else {
                return Err(Error::msg("Torrent platform not supported"));
            }
        }

        pub async fn add_torrent(&self, name: &str, id: &str) -> Result<String, String> {
            let tp = match &self.torrent_platform {
                TorrentPlatformsEnum::TorrentLeech(c) => {
                    c.download_torrent(name.to_string(), id.to_string()).await
                }
            };
            if let Ok(b64) = tp {
                match &self.torrent_client {
                    TorrentClientsEnum::Rtorrent(c) => c
                        .add_torrent_and_start(&b64, name.to_string())
                        .await
                        .expect("TODO: panic message"),
                    TorrentClientsEnum::Flood(c) => c
                        .add_torrent_and_start(&b64, name.to_string())
                        .await
                        .expect("TODO: panic message"),
                }
                return Ok(format!("Torrent {} added to rtorrent", name));
            }
            Err("Can not download torrent file".to_string())
        }

        pub async fn add_torrent_to_watchlist(&self, argument: String) -> Result<String, String> {
            self.options
                .borrow_mut()
                .add_dl_regex(argument.clone())
                .await;
            return Ok(format!("Torrent {} added to watch list", argument));
        }

        pub async fn remove_torrent_from_watchlist(&self, index: usize) -> Result<String, String> {
            if index < self.options.borrow().get_dl_regexes().len() {
                self.options.borrow_mut().remove_dl_regex(index).await;
                return Ok(format!("Torrent {} removed from watch list", index));
            }
            Err("Index out of range".to_string())
        }
    }
}
