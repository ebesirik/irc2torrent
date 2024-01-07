use crate::clients::TorrentClient;

pub struct rTorrent {
    host: String,
    port: u16,
    username: String,
    password: String,
}
impl TorrentClient for rTorrent {
    fn get_dl_list(&self) {
        todo!()
    }

    fn add_torrent_and_start(&self, file: String, name: String) {
        todo!()
    }

    fn add_torrent(&self, name: &str, id: &str) -> Result<String, String> {
        todo!()
    }
}