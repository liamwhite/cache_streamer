use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Config {
    #[arg(short, long, required = true)]
    /// Base URL to fetch against, such as "http://example.com"
    pub url: String,

    /// Address to bind to for serving HTTP.
    #[arg(short, long, default_value = "127.0.0.1:3000")]
    pub bind_address: String,

    /// Total capacity of the cache, in MiB.
    #[arg(short, long, default_value_t = 2048)]
    pub capacity: usize,

    /// Largest size for which an object can be cached, in MiB.
    /// Larger objects will be passed through instead.
    #[arg(short, long, default_value_t = 100)]
    pub limit: usize,
}
