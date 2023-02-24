use log::LevelFilter;
use simplelog::{ColorChoice, Config, TermLogger, TerminalMode};
use std::str::FromStr;

pub fn init(level_filter: &String) {
    TermLogger::init(
        LevelFilter::from_str(&level_filter).expect("Failed to parse logger level_filter"),
        Config::default(),
        TerminalMode::Stdout,
        ColorChoice::Auto,
    )
    .unwrap();
}
