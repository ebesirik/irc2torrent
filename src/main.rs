#[macro_use]
extern crate log;
extern crate pub_sub;
extern crate simplelog;
extern crate syslog;

use std::process;

use log::LevelFilter;
use simplelog::*;
use syslog::{BasicLogger, Facility, Formatter3164};
// use toml;

#[tokio::main]
async fn main() -> Result<(), failure::Error> {
    let formatter = Formatter3164 {
        facility: Facility::LOG_USER,
        hostname: None,
        process: "irc2torrent".into(),
        pid: process::id(),
    };
    if let Ok(logger) = syslog::unix(formatter) {
        let _ = log::set_boxed_logger(Box::new(BasicLogger::new(logger)))
            .map(|()| log::set_max_level(LevelFilter::Info));
    } else {
        CombinedLogger::init(vec![
            #[cfg(feature = "termcolor")]
                TermLogger::new(LevelFilter::Info, Config::default(), TerminalMode::Mixed, ColorChoice::Auto),
            #[cfg(not(feature = "termcolor"))]
                SimpleLogger::new(LevelFilter::Info, Config::default()),
        ]).unwrap();
    }
    info!("Started the app");
    let app = irc2torrent::Irc2Torrent::new();
    app.await.start().await;

    Ok(())
}


