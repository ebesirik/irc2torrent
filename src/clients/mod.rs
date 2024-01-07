pub mod rtorrent;
pub mod flood;

pub trait TorrentClient {
    fn get_dl_list(&self);
    fn add_torrent_and_start(&self, file: String, name: String);
    fn add_torrent(&self, name: &str, id: &str) -> Result<String, String>;
}