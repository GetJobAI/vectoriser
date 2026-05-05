use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "vectoriser", about = "Vector embedding microservice")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Start the service (consumer loop + healthz endpoint)
    Serve,
    /// Download and cache the BGEM3 ONNX model without starting the service
    DownloadModel,
}
