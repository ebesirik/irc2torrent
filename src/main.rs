mod torrent_processor;

use serde::{Serialize, Deserialize};
use std::fs;
use toml;
use irc::client::prelude::*;
use futures::prelude::*;
use regex::Regex;
use std::path::Path;
use directories::BaseDirs;
use crate::torrent_processor::TorrentProcessor;

#[derive(Serialize,Deserialize)]
struct Data {
    config: Config,
}

#[derive(Serialize,Deserialize)]
struct Config {
    rss_key: String,
    rtorrent_xmlrpc_url: String,
    regex_for_downloads_match: Vec<String>
}

#[tokio::main]
async fn main() -> Result<(), failure::Error> {
    let mut config = irc::client::data::config::Config::default();
    if let Some(proj_dir) = BaseDirs::new(){
        let cfg_dir_real = proj_dir.config_dir().join("irc2torrent/irc.toml");
        let cfg_dir_default = proj_dir.config_dir().join("irc2torrent/irc.defaults.toml");
        let config_paths = [
            cfg_dir_real.as_path(),
            Path::new("./irc.toml"),
            cfg_dir_default.as_path(),
            Path::new("./irc.defaults.toml"),
        ];
        for i in config_paths{
            if i.exists() {
                config = irc::client::data::config::Config::load(i)?;
                break;
            }
        }
        if !config_paths[0].exists(){
            let c_dir = proj_dir.config_dir().join("irc2torrent");
            if !c_dir.exists() {
                fs::create_dir(c_dir);
            }
            fs::copy(config_paths[3], config_paths[0]).expect("Unable to copy default file to its location.");
        }
    }

    let mut client = Client::from_config(config).await?;
    client.identify()?;

    let mut stream = client.stream()?;

    let re = Regex::new(r".*Name:'(?P<name>.*)' uploaded by.*https://www.torrentleech.org/torrent/(?P<id>\d+)").unwrap();

    let filename = "irc2torrent/options.toml";
    if let Some(options) = read_or_create_options(filename.to_string()){
        let processor = TorrentProcessor::new(options.config.rss_key, options.config.rtorrent_xmlrpc_url);
        let mut torrent_names_regexes: Vec<Regex> = Vec::new();
        for downloads_match in options.config.regex_for_downloads_match {
            if let Ok(dr) = Regex::new(downloads_match.as_str()){
                torrent_names_regexes.push(dr);
            }
        }
        while let Some(message) = stream.next().await.transpose()? {
            //println!("{}", message);
            let msg_str = message.to_string();
            if let Some(caps) = re.captures(msg_str.as_str()){
                let (name, id) = (&caps["name"] as &str, &caps["id"] as &str);
                println!("Torrent name: {}", name);
                println!("Torrent Id: {}", id);
                for regex in &torrent_names_regexes {
                    if regex.is_match(name) {
                        if let Ok(b64) = processor.download_torrent(name.to_string(), id.to_string()).await{
                            processor.add_torrent_and_start(b64, name.to_string()).await;
                        }
                        break;
                    }
                }
            }
        }
    }

    Ok(())
}

fn read_or_create_options(filename: String) -> Option<Data> {
    let mut result = Data {
        config: Config {
            rss_key: "XXXXXXXXXXXXXX".to_string(),
            rtorrent_xmlrpc_url: "http://127.0.0.1:5000/".to_string(),
            regex_for_downloads_match: [
                "Some Regex to match.*1080p.*".to_string(),
                "Another Release.*S02.*1080p.*WEB.*".to_string()
            ].to_vec()
        }
    };
    if let Some(proj_dir) = BaseDirs::new() {
        let dir = proj_dir.config_dir();
        let full_path_buf = dir.join(filename);

        if full_path_buf.exists() {
            let path = full_path_buf.as_path();
            let contents = match fs::read_to_string(path) {
                Ok(c) => c,
                Err(_) => {
                    eprintln!("Could not read file `{}`", path.to_str()?);
                    return None;
                }
            };
            result = match toml::from_str(&contents) {
                Ok(d) => d,
                Err(_) => {
                    eprintln!("Unable to load data from `{}`", path.to_str()?);
                    return None;
                }
            };
        } else {
            let toml = toml::to_string(&result).unwrap();
            let path = full_path_buf.as_path();
            match fs::write(path, toml) {
                Ok(_) => println!("New options file created at '{}' location, please consider modifying it before running to app.", path.to_str()?),
                Err(_) => eprintln!("Error creating {} file", path.to_str()?)
            };
        }
    }

    return Some(result);
}
