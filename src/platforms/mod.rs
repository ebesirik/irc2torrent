use std::path::PathBuf;

pub mod tl;

pub trait TorrentPlatform {
    fn get_rss_key(&self) -> String;
    fn get_torrent_files_dir(&self) -> PathBuf;
    async fn download_torrent(&self, name: String, id: String) -> Result<String, String>;
}