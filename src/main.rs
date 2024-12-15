use clap::Parser;
use config::Config;
use std::env;

mod config;
mod server;

fn main() {
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info")
    }

    env_logger::init();

    server::run(&Config::parse());
}
