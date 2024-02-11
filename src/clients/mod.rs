use base64::Engine;

pub mod rtorrent;
pub mod flood;

pub trait TorrentClient {
    async fn get_dl_list(&self) -> String;
    async fn add_torrent_and_start(&self, file: String, name: String);
    async fn add_torrent(&self, name: &str, id: &str) -> Result<String, String> {
        if let Ok(b64) = self.download_torrent(name.to_string(), id.to_string()).await {
            self.add_torrent_and_start(b64, name.to_string()).await;
            return Ok(format!("Torrent {} added to rtorrent", name));
        }
        Err("Can not download torrent file".to_string())
    }
}