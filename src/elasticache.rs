use crate::AppResult;
use aws_sdk_elasticache::Client;
use log::debug;

pub async fn list_replication_groups(client: &Client, cluster: &str) -> AppResult<Vec<String>> {
    let mut replication_groups = Vec::new();
    let mut replication_groups_stream = client
        .describe_replication_groups()
        .max_records(100)
        .into_paginator()
        .send();

    while let Some(replication_group) = replication_groups_stream.next().await {
        debug!("Replication Groups: {:?}", replication_group);
        for group in replication_group.unwrap().replication_groups.unwrap() {
            if group.replication_group_id().unwrap().contains(cluster)
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
) -> AppResult<()> {
    client
        .delete_replication_group()
        .replication_group_id(replication_group_id)
        .send()
        .await?;
    Ok(())
}
