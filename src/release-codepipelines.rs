use aws_config::Region;
use aws_sdk_codepipeline::Client as CodepipelineClient;

use aws_toolkit::{client::initialize_client, codepipeline, AppResult};
use clap::Parser;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[clap(
    version = "v0.1.0",
    author = "Anton Sidorov tonysidrock@gmail.com",
    about = "Release codepipeline"
)]
struct Args {
    
    #[clap(short, long, default_value = "eu-central-1")]
    region: String,

    #[clap(short, long, default_value = "default")]
    profile: String,

    #[clap(short = 'm', long, default_value = None)]
    prefix_match: Vec<String>,

    #[clap(short = 'x', long, default_value = None)]
    prefix_exclude: Vec<String>,

    #[clap(short, long)]
    failed_only: bool,

    #[clap(short, long)]
    print_failed_only: bool,
}

#[tokio::main]
async fn main() -> AppResult<()> {
    env_logger::init();

    let args = Args::parse();
    let region = Region::new(args.region.clone());

    let codepipeline_client =
        initialize_client::<CodepipelineClient>(region.clone(), &args.profile).await;
    let pipelines = codepipeline::list_pipelines(
        &codepipeline_client,
        &args.prefix_match,
        &args.prefix_exclude,
    )
    .await?;

    for pipeline in pipelines {
        println!("{}", pipeline);
    }

    Ok(())
}
