use std::error::Error;
use std::fmt;
use std::io::ErrorKind;
use std::path::PathBuf;
use log::info;
use reqwest::Url;
use serde_derive::Deserialize;
use crate::clients::TorrentClient;

struct Flood {
    client: reqwest::Client,
    username: String,
    password: String,
    url: String,
    dest: String,
}

//{"success":true,"username":"nomercy","level":10}
#[derive(Deserialize)]
struct LoginResponse {
    success: bool,
    username: String,
    level: i32,
}
#[derive(Debug)]
struct MyError {
    message: String,
}

impl fmt::Display for MyError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for MyError {}
impl Flood {
    pub fn new(username: String, password: String, url: String, dest: String) -> Self {
        let mut client = reqwest::Client::new();
        Self::login(&mut client, url.clone(), username.clone(), password.clone());
        Self { client: client, username, password, url, dest }
    }
    
    pub async fn login(client: &mut reqwest::Client, url: String, username: String, password: String) -> Result<(), Box<dyn Error>> {
        let resp = client.post(&format!("{}/api/auth/authenticate", url))
            .json(&serde_json::json!({
                "username": username,
                "password": password
            }))
            .header("Content-Type", "application/json")
            .send()
            .await?;
        info!("Login response: {:?}", resp);
        let login_response: LoginResponse = resp.json().await?;
        if login_response.success {
            info!("Login successful");
            Ok(())
        } else {
            info!("Login failed");
            Err(Box::new(MyError { message: "Login failed".to_string() }))
        }
    }
}

impl TorrentClient for Flood {
    async fn get_dl_list(&self) -> String {
        info!("Not implemented yet");
        "Not implemented yet".to_string()
    }

    async fn add_torrent_and_start(&self, file: String, name: String) {
        todo!()
    }

    async fn add_torrent(&self, name: &str, id: &str) -> Result<String, String> {
        todo!()
    }
}

#[cfg(test)]
mod test{
    use super::*;
    use tokio;
    #[tokio::test]
    async fn test_login() {
        let mut client = reqwest::Client::new();
        let url = "http://10.11.12.130:3000".to_string();
        let username = "".to_string();
        let password = "".to_string();
        let resp = Flood::login(&mut client, url.clone(), username.clone(), password.clone()).await;
        assert!(resp.is_ok());
    }
}