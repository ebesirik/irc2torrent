use crate::clients::TorrentClient;

pub struct Flood {
    pub name: String,
    pub age: u8,
    pub height: f32,
    pub weight: f32,
    pub is_alive: bool,
}

impl TorrentClient for Flood {
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