pub mod torrent {
    use std::fs::File;
    use std::io;
    //use std::ops::{Deref, DerefMut};
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};
    use base64;
    use base64::{Engine as _, engine::general_purpose};
    use dxr::client::{Client, ClientBuilder, Url, Call};
    use tokio::fs;
    use lava_torrent::torrent::v1::Torrent;
    use log::{error, info};
    use pub_sub::{PubSub, Subscription};
    use regex::Regex;
    use crate::config::config::Config;

    pub struct TorrentProcessor {
        evt_channel: PubSub<String>,
        subs_cfg: Vec<Subscription<String>>,
        rss_key: String,
        rtorrent_url: String,
        torrent_files_dir: PathBuf,
        torrent_match_regex_list: Vec<Regex>,
        options: Arc<Mutex<Config>>,
    }

    impl TorrentProcessor {
        pub fn new(dir: PathBuf, config: Arc<Mutex<Config>>, evt_channel: PubSub<String>, subs_cfg: Vec<Subscription<String>>) -> Self {
            let cfg = config.lock().unwrap();
            Self { evt_channel, subs_cfg, rss_key: cfg.get_rss_key(), rtorrent_url: cfg.get_xmlrpc_url(), torrent_files_dir: dir.join("torrent_files/"), torrent_match_regex_list: cfg.get_dl_regexes(), options: config.clone() }
        }

        pub fn do_we_want_this_torrent(&self, name: &String) -> bool {
            for regex in &self.torrent_match_regex_list {
                if regex.is_match(name) {
                    return true;
                }
            }
            return false;
        }

        pub async fn get_dl_list(&self) {
            let url = Url::parse(&self.rtorrent_url).unwrap();
            let client: Client = ClientBuilder::new(url)
                .user_agent("dxr-client-example")
                .build();
            let request = Call::new("d.multicall2", ("main", "d.get_name=", "d.get_base_path=", "d.get_size_bytes=", "d.get_creation_date=", "d.get_custom=addtime="));
            let result = client.call(request).await as Result<String, anyhow::Error>;
            match result {
                Ok(r) => {
                    info!("Torrent load result: {r:?}");
                }
                Err(e) => {
                    error!("Error loading torrent: {:?}", e);
                }
            }
        }

        pub async fn add_torrent_and_start(&self, file: String, name: String) {
            let url = Url::parse(&self.rtorrent_url).unwrap();
            let client: Client = ClientBuilder::new(url)
                .user_agent("dxr-client-example")
                .build();
            //println!("{}", file);
            if let Ok(bytes) = &general_purpose::STANDARD_NO_PAD.decode(file.as_bytes()) {
                let request = Call::new("load.raw_start_verbose", ("", bytes.as_slice()));
                // let request = dxr::client::Call::new("load.raw_start_verbose", file);
                let result = client.call(request).await as Result<i32, anyhow::Error>;
                let hasher = Torrent::read_from_bytes(bytes).unwrap();
                let the_hash = hasher.info_hash();
                match result {
                    Ok(r) => {
                        info!("Torrent load result: ({name}) {r:?}");
                        info!("Torrent Hash for time fix: {the_hash}");
                        /*
                        (https://www.reddit.com/r/torrents/comments/ejkqjv/does_rtorrent_have_knowledge_of_a_date_added/)
                        this line should be added to your .rtorrent.rc file for next line to work;
                        method.insert = fix_addtime, simple, "d.custom.set=addtime,(cat,$d.creation_date=)"
                        */
                        let fix_time: Call<String, i32> = Call::new("fix_addtime", the_hash);
                        match client.call(fix_time).await {
                            Ok(_) => info!("Time fixed for {}", name),
                            Err(e2) => error!("Error fixing time {:?},\n\t did you forget to add this line: \n\\t\t'{}'\n\t to your .rtorrent.rc file?", e2, "method.insert = fix_addtime, simple, \"d.custom.set=addtime,(cat,$d.creation_date=)\"")
                        }
                    }
                    Err(e) => println!("File upload err: ({}) {:?}", name, e),
                }
            }
        }

        pub async fn download_torrent(&self, name: String, id: String) -> Result<String, String> {
            println!("Downloading torrent: {}", name);

            let torrent_file = name.replace(" ", ".") + ".torrent";
            if let Ok(resp) = reqwest::get(format!("https://www.torrentleech.org/rss/download/{}/{}/{}", id, &self.rss_key, torrent_file)).await {
                if let Ok(bytes) = resp.bytes().await {
                    if !self.torrent_files_dir.exists() {
                        let _ = fs::create_dir(&self.torrent_files_dir).await;
                    }
                    let mut slice: &[u8] = bytes.as_ref();
                    let mut out = File::create(self.torrent_files_dir.join(torrent_file)).expect("Failed file create");
                    let _ = io::copy(&mut slice, &mut out);
                    return Ok(general_purpose::STANDARD_NO_PAD.encode(bytes.as_ref()));
                }
            }
            Err("Failed to download file".to_string())
        }

        pub async fn add_torrent(&self, name: &str, id: &str) -> Result<String, String> {
            if let Ok(b64) = self.download_torrent(name.to_string(), id.to_string()).await {
                self.add_torrent_and_start(b64, name.to_string()).await;
                return Ok(format!("Torrent {} added to rtorrent", name));
            }
            Err("Can not download torrent file".to_string())
        }

        pub async fn add_torrent_to_watchlist(&mut self, argument: String) -> Result<String, String> {
            let err_str = "Regex format error. Use: addtowatchlist: <regex>";
            if let Ok(rgx) = Regex::new(argument.as_str()) {
                self.torrent_match_regex_list.push(rgx);
                self.options.lock().unwrap().add_dl_regex(argument.clone()).await;
                return Ok(format!("Torrent {} added to watch list", argument));
            }
            Err(err_str.to_string())
        }

        pub fn remove_torrent_from_watchlist(&mut self, index: usize) -> Result<String, String> {
            if index < self.torrent_match_regex_list.len() {
                self.torrent_match_regex_list.remove(index);
                return Ok(format!("Torrent {} removed from watch list", index));
            }
            Err("Index out of range".to_string())
        }
    }
}
