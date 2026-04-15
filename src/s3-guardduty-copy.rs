use aws_config::Region;
use aws_sdk_guardduty::Client as GDClient;
use aws_sdk_s3::Client as S3Client;
use log::debug;

use aws_toolkit::{AppResult, client::initialize_client, guardduty, s3};
use clap::Parser;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[clap(
    version = "v0.1.0",
    author = "Anton Sidorov tonysidrock@gmail.com",
    about = "s3-guardduty-copy"
)]
struct Args {
    #[clap(short, long, default_value = "eu-central-1")]
    region: String,

    #[clap(short, long, default_value = "default")]
    profile: String,

    #[clap(short = 'm', long, default_value = "")]
    prefix_match: Vec<String>,

    #[clap(short = 'x', long, default_value = None)]
    prefix_exclude: Vec<String>,

    #[clap(short, long)]
    all: bool,

    #[clap(short, long)]
    dry_run: bool,
}

#[tokio::main]
async fn main() -> AppResult<()> {
    env_logger::init();

    let args = Args::parse();
    let region = Region::new(args.region.clone());

    let s3_client = initialize_client::<S3Client>(region.clone(), &args.profile).await;
    let gd_client = initialize_client::<GDClient>(region.clone(), &args.profile).await;

    let bucket_names: Vec<String> = if args.all {
        s3::get_buckets(&s3_client, &args.prefix_match, &args.prefix_exclude).await?
    } else {
        guardduty::list_malware_protected_buckets(
            &gd_client,
            &args.prefix_match,
            &args.prefix_exclude,
        )
        .await?
    };
    debug!("{:?}", bucket_names);

    for bucket in bucket_names {
        print!("Bucket {}: ", &bucket);
        let bucket_policy = s3::save_bucket_policy(&s3_client, &bucket).await?;
        if bucket_policy.is_some() {
            s3_client
                .delete_bucket_policy()
                .bucket(&bucket)
                .send()
                .await?;
        }

        let mut counter = 0;
        let objects = s3::list_all_objects(&s3_client, &bucket).await?;

        for key in objects {
            if !&args.dry_run
                && let Err(e) = s3::update_metadata_in_place(&s3_client, &bucket, &key).await
            {
                eprintln!(
                    "Error: file '{}' could not be copied to bucket '{}'. Details: {}",
                    key, bucket, e
                );
            };
            counter += 1;
            if counter % 100 == 0 {
                println!("copied {} files", counter);
            }
        }
        if args.dry_run {
            println!("{}copied {} files", &"dry run, would be ", counter);
        } else {
            println!("copied {} files", counter);
        }
        if let Some(policy) = bucket_policy {
            s3::restore_bucket_policy(&s3_client, &bucket, &policy).await?;
        }
    }

    Ok(())
}
