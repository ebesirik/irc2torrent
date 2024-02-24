use std::path::PathBuf;
use anyhow::Error;

pub mod tl;

pub trait TorrentPlatform {
    fn get_torrent_files_dir(&self) -> &PathBuf;
    async fn download_torrent(&self, name: String, id: String) -> Result<String, Error>;
}