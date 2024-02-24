use std::fmt::Debug;
use anyhow::Error;
use base64::Engine;
use chrono::{DateTime, Local};

pub mod rtorrent;
pub mod flood;

pub trait TorrentClient {
    async fn get_dl_list(&mut self) -> Result<Vec<DownloadResult>, Error>;
    async fn add_torrent_and_start(&mut self, file: String, name: String) -> Result<(), Error>;
    /*async fn add_torrent(&self, name: &str, id: &str) -> Result<String, String> {
        if let Ok(b64) = self.download_torrent(name.to_string(), id.to_string()).await {
            self.add_torrent_and_start(b64, name.to_string()).await;
            return Ok(format!("Torrent {} added to rtorrent", name));
        }
        Err("Can not download torrent file".to_string())
    }*/
}

#[derive(serde::Deserialize, serde::Serialize, Clone, PartialEq, Eq, Hash, Default)]
struct DownloadResult {
    name: String,
    size: i64,
    creation_date: i64,
}

impl DownloadResult {
    pub fn get_utc_creation_date(&self) -> DateTime<Local> {
        let datetime: DateTime<Local> = DateTime::from(DateTime::from_timestamp(self.creation_date, 0).unwrap());
        datetime
    }
    pub fn get_readable_size(&self) -> String {
        let mut size = self.size as f64;
        let units = ["B", "KB", "MB", "GB", "TB", "PB", "EB", "ZB", "YB"];
        let mut i = 0;
        while size > 1024.0 {
            i += 1;
            size = size / 1024.0;
        }
        format!("{:.2}{}", size, units[i])
    }
}

impl ToString for DownloadResult {
    fn to_string(&self) -> String {
        format!("Name: {}, Size: {}, Creation Date: {}", self.name, self.get_readable_size(), self.get_utc_creation_date())
    }
}

impl Debug for DownloadResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Name: {}, Size: {}, Creation Date: {}", self.name, self.get_readable_size(), self.get_utc_creation_date())
    }
}