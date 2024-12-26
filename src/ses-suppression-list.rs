use aws_config::Region;
use aws_sdk_sesv2::Client as Sesv2Client;
use aws_toolkit::{client::initialize_client, sesv2, Result};

use clap::Parser;
use log::debug;
use std::fs::File;
use std::io::Write;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[clap(
    version = "v0.2.2",
    author = "Anton Sidorov tonysidrock@gmail.com",
    about = "Exports ses suppression list"
)]
struct Args {
    #[clap(short, long, default_value = "eu-central-1")]
    region: String,

    #[clap(short, long, default_value = "suppressed_emails.csv")]
    output: String,

    #[clap(short, long, default_value = "default")]
    profile: String,

    #[clap(short, long, default_value = None)]
    last: Option<u32>,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let args = Args::parse();
    let region = Region::new(args.region.clone());

    let sesv2_client = initialize_client::<Sesv2Client>(region, &args.profile).await;

    if let Ok(r) = sesv2::get_suppression_list(&sesv2_client, args.last).await {
        debug!("Result: {:?}", &r);
        let mut file = File::create(format!("./{}", &args.output)).unwrap();

        for (email, reason, date) in &r {
            writeln!(file, "{},{},{}", email, reason, date).unwrap();
        }
        println!("Total {} email addresses", r.len())
    }

    Ok(())
}
