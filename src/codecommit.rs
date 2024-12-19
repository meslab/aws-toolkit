use aws_config::default_provider::credentials::DefaultCredentialsChain;
use aws_sdk_codecommit::config::Region;
use aws_sdk_codecommit::{Client, Config};
use log::debug;

pub async fn initialize_client(region: &str, profile: &str) -> Client {
    let codecommit_region = Region::new(region.to_owned());

    let credentials_provider = DefaultCredentialsChain::builder()
        .region(codecommit_region.clone())
        .profile_name(profile)
        .build()
        .await;
    
    let config = Config::builder()
        .credentials_provider(credentials_provider)
        .region(codecommit_region)
        .build();

    Client::from_conf(config)
}

async fn list_filtered_repositories_internal<F>(
    client: &Client,
    filter: F,
) -> Result<Vec<String>, aws_sdk_codecommit::Error>
where
    F: Fn(&str) -> bool + Send + Sync,
{
    let mut repos = Vec::new();
    
    let mut repos_stream = client.list_repositories().into_paginator().send();
    
    while let Some(output) = repos_stream.next().await {
        for repo in output?.repositories.unwrap_or_default() {
            if let Some(repo_name) = repo.repository_name {
                if filter(&repo_name) {
                    repos.push(repo_name);
                }
            }
        }
    }
    
    debug!("Repositories: {:?}", repos);
    Ok(repos)
}

pub async fn list_repositories(
    client: &Client,
    include: &[String],
    exclude: &[String],
) -> Result<Vec<String>, aws_sdk_codecommit::Error> {
    let include: Vec<_> = include.iter().map(|x| x.as_str()).collect();
    let exclude: Vec<_> = exclude.iter().map(|x| x.as_str()).collect();
    
    let filter = |repo_name: &str| {
        include.iter().any(|&x| repo_name.contains(x))
            && exclude.iter().all(|&x| !repo_name.contains(x))
    };
    
    list_filtered_repositories_internal(client, filter).await
}

pub async fn list_exact_repositories(
    client: &Client,
    include: &[String],
    exclude: &[String],
) -> Result<Vec<String>, aws_sdk_codecommit::Error> {
    let include: Vec<_> = include.iter().map(|x| x.as_str()).collect();
    let exclude: Vec<_> = exclude.iter().map(|x| x.as_str()).collect();
    
    let filter = |repo_name: &str| {
        include.iter().any(|&x| x == repo_name)
            && exclude.iter().all(|&x| x != repo_name)
    };
    
    list_filtered_repositories_internal(client, filter).await
}
