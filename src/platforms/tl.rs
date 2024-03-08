use std::fs::File;
use std::io;
use anyhow::Error;
use std::path::PathBuf;
use base64::Engine;
use base64::engine::general_purpose;
use log::info;
use tokio::fs;
use crate::platforms::TorrentPlatform;

pub(crate) struct TorrentLeech {
    rss_key: String,
    torrent_dir: PathBuf,
}

impl TorrentLeech {
    pub fn new(rss_key: String, torrent_dir: String) -> Self {
        let td = PathBuf::from(torrent_dir);
        Self { rss_key, torrent_dir: td }
    }
}

impl TorrentPlatform for TorrentLeech {
    fn get_torrent_files_dir(&self) -> &PathBuf {
        &self.torrent_dir
    }

    async fn download_torrent(&self, name: String, id: String) -> Result<String, Error> {
        info!("Downloading torrent: {}", name);

        let torrent_file = name.replace(" ", ".") + ".torrent";
        if let Ok(resp) = reqwest::get(format!("https://www.torrentleech.org/rss/download/{}/{}/{}", id, &self.rss_key, torrent_file)).await {
            if let Ok(bytes) = resp.bytes().await {
                if !self.get_torrent_files_dir().exists() {
                    let _ = fs::create_dir(&self.get_torrent_files_dir()).await?;
                }
                let mut slice: &[u8] = bytes.as_ref();
                let mut out = File::create(self.get_torrent_files_dir().join(torrent_file)).expect("Failed file create");
                let _ = io::copy(&mut slice, &mut out);
                return Ok(general_purpose::STANDARD_NO_PAD.encode(bytes.as_ref()));
            }
        }
        Err(Error::msg("Failed to download file"))
    }
}