use crate::AppResult;
use aws_sdk_codecommit::Client;
use log::debug;

async fn list_filtered_repositories_internal<F>(
    client: &Client,
    filter: F,
) -> AppResult<Vec<String>>
where
    F: Fn(&str) -> bool + Send + Sync,
{
    let mut repos = Vec::new();

    let mut repos_stream = client.list_repositories().into_paginator().send();

    while let Some(output) = repos_stream.next().await {
        repos.extend(output?.repositories().iter().filter_map(|repo| {
            let repo_name = repo.repository_name()?;
            if filter(repo_name) {
                Some(repo_name.to_owned())
            } else {
                None
            }
        }));
    }

    debug!("Repositories: {:?}", repos);
    Ok(repos)
}

pub async fn list_repositories(
    client: &Client,
    include: &[String],
    exclude: &[String],
) -> AppResult<Vec<String>> {
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
) -> AppResult<Vec<String>> {
    let include: Vec<_> = include.iter().map(|x| x.as_str()).collect();
    let exclude: Vec<_> = exclude.iter().map(|x| x.as_str()).collect();

    let filter = |repo_name: &str| {
        include.iter().any(|&x| x == repo_name) && exclude.iter().all(|&x| x != repo_name)
    };

    list_filtered_repositories_internal(client, filter).await
}
