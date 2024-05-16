#[macro_use]
extern crate log;
extern crate pub_sub;
extern crate simplelog;
extern crate syslog;

use std::process;

use log::LevelFilter;
use simplelog::*;
use syslog::{Facility, Formatter3164};
// use toml;

#[tokio::main]
async fn main() -> Result<(), failure::Error> {
    CombinedLogger::init(vec![
        #[cfg(all(feature = "termcolor", not(debug_assertions)))]
            TermLogger::new(LevelFilter::Info, Config::default(), TerminalMode::Mixed, ColorChoice::Auto),
        #[cfg(all(not(feature = "termcolor"), not(debug_assertions)))]
            SimpleLogger::new(LevelFilter::Info, Config::default()),
        #[cfg(debug_assertions)]
            TestLogger::new(LevelFilter::Info, Default::default()),
    ]).unwrap();
    info!("Started the app");
    let mut app = irc2torrent::Irc2Torrent::new().await;
    app.start().await;

    Ok(())
}


