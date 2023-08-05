use std::fs::File;
use std::io;
use std::path::PathBuf;
use base64;
use dxr::client::{Client, ClientBuilder, Url, Call};
use tokio::fs;
// use openssl::sha::Sha1;
// use hex_literal::hex;
use lava_torrent::torrent::v1::Torrent;
use regex::Regex;
use crate::config::Config;

pub struct TorrentProcessor<'tp> {
    rss_key: String,
    rtorrent_url: String,
    torrent_files_dir: PathBuf,
    torrent_match_regex_list: Vec<Regex>,
    options: &'tp Config
}

impl <'tp>TorrentProcessor<'tp> {

    pub fn new(rss:String, url: String, dir: PathBuf, config: &'tp Config) -> Self{
        Self { rss_key: rss, rtorrent_url: url, torrent_files_dir: dir.join("torrent_files/"), torrent_match_regex_list: config.get_dl_regexes(), options: config }
    }
    
    pub fn do_we_want_this_torrent(&self, name: &String) -> bool {
        for regex in &self.torrent_match_regex_list {
            if regex.is_match(name) {
                return true;
            }
        }
        return false;
    }

    pub async fn get_dl_list(&self){
        let url = Url::parse(&self.rtorrent_url).unwrap();
        let client: Client = ClientBuilder::new(url)
            .user_agent("dxr-client-example")
            .build();
        let request = Call::new("d.multicall2", ("main", "d.get_name=", "d.get_base_path=", "d.get_size_bytes=", "d.get_creation_date=", "d.get_custom=addtime="));
        let result = client.call(request).await as Result<String, anyhow::Error>;
        match result {
            Ok(r) => {
                info!("Torrent load result: {r:?}");
            },
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
        if let Ok(bytes) = base64::decode(file){
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
                    let fix_time : Call<String, i32> = Call::new("fix_addtime", the_hash);
                    match client.call(fix_time).await {
                        Ok(_) => info!("Time fixed for {}", name),
                        Err(e2) => error!("Error fixing time {:?},\n\t did you forget to add this line: \n\\t\t'{}'\n\t to your .rtorrent.rc file?", e2, "method.insert = fix_addtime, simple, \"d.custom.set=addtime,(cat,$d.creation_date=)\"")
                    }
                },
                Err(e) => println!("File upload err: ({}) {:?}", name, e),
            }
        }
    }

    pub async fn download_torrent(&self, name: String, id: String) -> Result<String, String> {
        println!("Downloading torrent: {}", name);
        // let dl_key = "7ef1038ba2293421b526".to_string();

        let torrent_file = name.replace(" ", ".") + ".torrent";
        if let Ok(resp) = reqwest::get(format!("https://www.torrentleech.org/rss/download/{}/{}/{}", id, &self.rss_key, torrent_file)).await {
            if let Ok(bytes) = resp.bytes().await{
                if !self.torrent_files_dir.exists() {
                    let _ = fs::create_dir(&self.torrent_files_dir).await;
                }
                let mut slice: &[u8] = bytes.as_ref();
                let mut out = File::create(self.torrent_files_dir.join(torrent_file)).expect("Failed file create");
                let _ = io::copy(&mut slice, &mut out);
                return Ok(base64::encode(bytes.as_ref()));
            }
        }
        Err("Failed to download file".to_string())
    }
}