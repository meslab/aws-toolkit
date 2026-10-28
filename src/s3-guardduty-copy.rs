use aws_config::Region;
use aws_sdk_guardduty::Client as GDClient;
use aws_sdk_s3::Client as S3Client;
use log::debug;

use aws_toolkit::{AppResult, client::initialize_client, guardduty, s3};
use clap::Parser;

#[derive(Parser)]
#[command(
    author,
    version,
    about = "s3-guardduty-copy",
    long_about = None
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
        println!("Bucket {}: ", bucket);

        let bucket_policy = if args.dry_run {
            None
        } else {
            let policy = s3::save_bucket_policy(&s3_client, &bucket).await?;
            if policy.is_some() {
                s3_client
                    .delete_bucket_policy()
                    .bucket(&bucket)
                    .send()
                    .await?;
            }
            policy
        };

        let copy_result = s3::copy_all_objects(&s3_client, &bucket, args.dry_run).await;

        if let Some(policy) = &bucket_policy
            && let Err(error) = s3::restore_bucket_policy(&s3_client, &bucket, policy).await
        {
            eprintln!("Failed to restore policy for bucket {}: {}", bucket, error);
            return Err(error);
        }

        let counter = copy_result?;
        if args.dry_run {
            println!("{} files would be copied", counter);
        } else {
            println!("copied {} files", counter);
        }
    }

    Ok(())
}
