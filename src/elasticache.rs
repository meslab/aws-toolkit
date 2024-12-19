use aws_config::default_provider::credentials::DefaultCredentialsChain;
use aws_sdk_elasticache::config::Region;
use aws_sdk_elasticache::{Client, Config};
use log::debug;

pub async fn initialize_client(region: &str, profile: &str) -> Client {
    let credentials_provider = DefaultCredentialsChain::builder()
        .profile_name(profile)
        .build()
        .await;
    let config = Config::builder()
        .credentials_provider(credentials_provider)
        .region(Region::new(region.to_owned()))
        .build();
    Client::from_conf(config)
}

pub async fn list_replication_groups(
    client: &Client,
    cluster: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut replication_groups = Vec::new();
    let mut replication_groups_stream = client
        .describe_replication_groups()
        .max_records(100)
        .into_paginator()
        .send();

    while let Some(replication_group) = replication_groups_stream.next().await {
        debug!("Replication Groups: {:?}", replication_group);
        for group in replication_group.unwrap().replication_groups.unwrap() {
            if group
                .replication_group_id
                .as_ref()
                .unwrap()
                .contains(cluster)
                && group.status.unwrap().contains("available")
            {
                replication_groups.push(group.replication_group_id.unwrap());
            }
        }
    }
    Ok(replication_groups)
}

pub async fn delete_replication_group(
    client: &Client,
    replication_group_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    client
        .delete_replication_group()
        .replication_group_id(replication_group_id)
        .send()
        .await?;
    Ok(())
}
