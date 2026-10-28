use aws_config::Region;
use aws_sdk_codecommit::Client as CodeCommitClient;
use aws_toolkit::{AppResult, client::initialize_client, codecommit};
use clap::Parser;
use git2::{Config, ConfigLevel};
use log::info;
use std::fs::File;
use std::io::Write;
use std::path::Path;

#[derive(Parser)]
#[command(
    author,
    version,
    about = "Updates custom gitconfig file",
    long_about = None
)]
struct Args {
    #[clap(short, long, default_value = "/tmp/gitconfig")]
    file: String,

    #[clap(short, long, required = true)]
    base: Vec<String>,

    #[clap(short, long, default_value = None)]
    include: Vec<String>,

    #[clap(short = 'x', long, default_value = None)]
    exclude: Vec<String>,

    #[clap(short, long, default_values = &["infra"])]
    profiles: Vec<String>,

    #[clap(short, long, default_values = &["eu-central-1", "us-east-2"])]
    regions: Vec<String>,

    #[clap(short, long, default_value = None)]
    email: Option<String>,

    #[clap(short, long, default_value = None)]
    name: Option<String>,
}

fn write_gitconfig(file: &mut File, repo: &str, region: &str, profile: &str) -> AppResult<()> {
    writeln!(
        file,
        "[credential \"https://git-codecommit.{}.amazonaws.com/v1/repos/{}.git\"]",
        region, repo
    )?;
    writeln!(
        file,
        "\thelper = !aws codecommit credential-helper $@ --profile {}",
        profile
    )?;
    writeln!(file, "\tuseHttpPath = true")?;
    Ok(())
}

#[tokio::main]
async fn main() -> AppResult<()> {
    env_logger::init();

    let args = Args::parse();

    let mut cfg = Config::open_default()?;
    let _ = cfg.add_file(Path::new(&args.file), ConfigLevel::Local, true);

    let username = match &args.name {
        Some(name) => name.clone(),
        None => cfg
            .get_string("user.name")
            .map_err(|_| "user.name is not configured; pass --name")?,
    };
    let email = match &args.email {
        Some(email) => email.clone(),
        None => cfg
            .get_string("user.email")
            .map_err(|_| "user.email is not configured; pass --email")?,
    };

    let mut file = File::create(&args.file)?;

    for profile in args.profiles.iter() {
        for region in args.regions.iter() {
            let codecommit_region = Region::new(region.clone());

            let client = initialize_client::<CodeCommitClient>(codecommit_region, profile).await;
            let base_repositories =
                codecommit::list_exact_repositories(&client, &args.base, &args.exclude).await?;
            info!("Base repositories: {:?}", base_repositories);
            let repositories =
                codecommit::list_repositories(&client, &args.include, &args.exclude).await?;
            info!("Repositories: {:?}", repositories);
            for repo in base_repositories {
                write_gitconfig(&mut file, &repo, region, profile)?;
            }
            for repo in repositories {
                write_gitconfig(&mut file, &repo, region, profile)?;
            }
        }
    }

    writeln!(
        file,
        "[credential]\n\thelper = !aws codecommit credential-helper $@\n\tUseHttpPath = true"
    )?;
    writeln!(file, "[user]\n\temail = {}\n\tname = {}", email, username)?;

    Ok(())
}
