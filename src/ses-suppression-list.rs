use aws_config::Region;
use aws_sdk_sesv2::Client as Sesv2Client;
use aws_toolkit::{AppResult, client::initialize_client, sesv2};

use clap::Parser;
use log::debug;
use std::fs::File;
use std::io::Write;

#[derive(Parser)]
#[command(
    author,
    version,
    about = "Exports ses suppression list",
    long_about = None
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

    #[clap(short, long, help = "Include suppression date")]
    full: bool,
}

#[tokio::main]
async fn main() -> AppResult<()> {
    env_logger::init();

    let args = Args::parse();
    let region = Region::new(args.region.clone());

    let sesv2_client = initialize_client::<Sesv2Client>(region, &args.profile).await;

    let suppression_list = sesv2::get_suppression_list(&sesv2_client, args.last).await?;
    debug!("Result: {:?}", suppression_list);
    let mut file = File::create(&args.output)?;

    for (email, reason, date) in &suppression_list {
        if args.full {
            writeln!(
                file,
                "{},{},{}",
                csv_escape(email),
                csv_escape(reason),
                csv_escape(date)
            )?;
        } else {
            writeln!(file, "{},{}", csv_escape(email), csv_escape(reason))?;
        }
    }
    println!("Total {} email addresses", suppression_list.len());

    Ok(())
}

fn csv_escape(value: &str) -> String {
    if value
        .chars()
        .any(|character| matches!(character, ',' | '"' | '\n' | '\r'))
    {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::csv_escape;

    #[test]
    fn leaves_plain_values_unchanged() {
        assert_eq!(csv_escape("user@example.com"), "user@example.com");
    }

    #[test]
    fn quotes_values_with_commas() {
        assert_eq!(csv_escape("user,test"), "\"user,test\"");
    }

    #[test]
    fn doubles_embedded_quotes() {
        assert_eq!(csv_escape("user\"test"), "\"user\"\"test\"");
    }
}
