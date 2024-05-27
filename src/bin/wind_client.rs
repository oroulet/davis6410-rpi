use anyhow::Result;

use clap::{Parser, Subcommand};
use davis_rpi::db::Measurement;

#[derive(Parser, Debug)]
#[command(name = "Wind server client")]
#[command(author = "Olivier")]
#[command(version = "0.1")]
#[command(about = "test request wind server", version = "1.0")]
struct Args {
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Current,
    Oldest,
    LastEvents { count: usize },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    match &args.command {
        Commands::Current => query("localhost".into(), "current".into(), None).await?,
        Commands::Oldest => query("localhost".into(), "oldest_data".into(), None).await?,
        Commands::LastEvents { count } => {
            query(
                "localhost".into(),
                "last_events".into(),
                Some(vec![format!("{count}")]),
            )
            .await?
        }
    };

    Ok(())
}

async fn query(host: String, cmd: String, _args: Option<Vec<String>>) -> Result<()> {
    let m = reqwest::get(format!("http://{host}:8080/wind/{cmd}"))
        .await?
        .json::<Measurement>()
        .await?;
    println!("{}", m.pretty_str());
    Ok(())
}
