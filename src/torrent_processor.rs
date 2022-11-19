use base64;
use dxr::client::{Client, ClientBuilder, Url, Call};

pub struct TorrentProcessor {
    rss_key: String,
    rtorrent_url: String
}

impl TorrentProcessor {

    pub fn new(rss:String, url: String) -> Self{
        Self { rss_key: rss, rtorrent_url: url}
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
            match result {
                Ok(r) => println!("Torrent load result: ({name}) {r:?}"),
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
                return Ok(base64::encode(bytes.as_ref()));
                /*let mut slice: &[u8] = bytes.as_ref();
                let mut out = File::create(torrent_file).expect("Failed file create");
                io::copy(&mut slice, &mut out);*/
            }
        }
        Err("Failed to download file".to_string())
    }
}