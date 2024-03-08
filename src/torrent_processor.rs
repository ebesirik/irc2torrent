pub mod torrent {
    use std::sync::{Arc, Mutex};

    use anyhow::Error;
    use base64;
    use base64::Engine as _;
    use pub_sub::{PubSub, Subscription};

    use crate::clients::{DownloadResult, TorrentClientsEnum};
    use crate::config::config::Config;
    use crate::platforms::{TorrentPlatform, TorrentPlatformsEnum};
    use crate::platforms::TorrentPlatformsEnum::TorrentLeech;

    pub struct TorrentProcessor
    {
        evt_channel: PubSub<String>,
        subs_cfg: Vec<Subscription<String>>,
        torrent_client: TorrentClientsEnum,
        torrent_platform: TorrentPlatformsEnum,
        options: Arc<Mutex<Config>>,
    }

    impl TorrentProcessor
    {
        pub fn new(
            config: Arc<Mutex<Config>>,
            evt_channel: PubSub<String>,
            subs_cfg: Vec<Subscription<String>>,
            torrent_client: TorrentClientsEnum,
            torrent_platform: TorrentPlatformsEnum,
        ) -> TorrentProcessor {
            Self {
                evt_channel,
                subs_cfg,
                torrent_client,
                torrent_platform,
                options: config,
            }
        }

        pub fn do_we_want_this_torrent(&self, name: &String) -> bool {
            let torrent_match_regex_list = self.options.lock().unwrap().get_dl_regexes();
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
                TorrentClientsEnum::Flood(c) => c.get_dl_list().await?
            };
            Ok(list.to_owned())
        }

        pub async fn add_torrent_and_start(&mut self, file: String, name: String) {
            match &self.torrent_client {
                TorrentClientsEnum::Rtorrent(c) => { c.add_torrent_and_start(&file, name).await.expect("TODO: panic message")},
                TorrentClientsEnum::Flood(c) => { c.add_torrent_and_start(&file, name).await.expect("TODO: panic message") }
            };
        }

        pub async fn download_torrent(&self, name: String, id: String) -> Result<String, Error> {
            if let TorrentLeech(tl) = &self.torrent_platform {
                return Ok(tl.download_torrent(name, id).await?);
            } else { 
                return Err(Error::msg("Torrent platform not supported"));
            }
        }

        pub async fn add_torrent(&mut self, name: &str, id: &str) -> Result<String, String> {
            let tp = match &self.torrent_platform {
                TorrentPlatformsEnum::TorrentLeech(c) => c.download_torrent(name.to_string(), id.to_string())
                    .await,
            };
            if let Ok(b64) = tp
            {
                match &self.torrent_client {
                    TorrentClientsEnum::Rtorrent(c) => c.add_torrent_and_start(&b64, name.to_string())
                        .await.expect("TODO: panic message"),
                    TorrentClientsEnum::Flood(c) => c.add_torrent_and_start(&b64, name.to_string())
                        .await.expect("TODO: panic message")
                }
                return Ok(format!("Torrent {} added to rtorrent", name));
            }
            Err("Can not download torrent file".to_string())
        }

        pub async fn add_torrent_to_watchlist(
            &mut self,
            argument: String,
        ) -> Result<String, String> {
            self.options
                .lock()
                .unwrap()
                .add_dl_regex(argument.clone())
                .await;
            return Ok(format!("Torrent {} added to watch list", argument));
        }

        pub fn remove_torrent_from_watchlist(&mut self, index: usize) -> Result<String, String> {
            if index < self.options.lock().unwrap().get_dl_regexes().len() {
                self.options.lock().unwrap().remove_dl_regex(index);
                return Ok(format!("Torrent {} removed from watch list", index));
            }
            Err("Index out of range".to_string())
        }

    }
}
