use crate::{AppResult, sanitize_string};
use aws_sdk_rds::{Client, types::Filter};
use chrono::{self, Utc};
use log::debug;

pub async fn list_db_instances(client: &Client, cluster: &str) -> AppResult<Vec<String>> {
    let mut db_instances = Vec::new();

    let filter_value = format!("{}-postgres", cluster);
    let filter = Filter::builder()
        .name("db-instance-id") // same as Name field in boto3
        .values(filter_value)
        .build();

    let mut db_instances_stream = client
        .describe_db_instances()
        .filters(filter)
        .max_records(100)
        .into_paginator()
        .send();

    while let Some(db_instances_output) = db_instances_stream.next().await {
        debug!("DB Instances: {:?}", db_instances_output);

        db_instances.extend(
            db_instances_output?
                .db_instances()
                .iter()
                .filter_map(|instance| {
                    let id = instance.db_instance_identifier()?;

                    if !id.contains(cluster) {
                        return None;
                    }

                    if !["available", "stopped"].contains(&instance.db_instance_status()?) {
                        return None;
                    };

                    Some(id.to_owned())
                }),
        );
    }

    Ok(db_instances)
}

pub async fn disable_deletion_protection(client: &Client, db_instance_id: &str) -> AppResult<()> {
    client
        .modify_db_instance()
        .db_instance_identifier(db_instance_id)
        .set_deletion_protection(Some(false))
        .apply_immediately(true)
        .send()
        .await?;
    Ok(())
}

pub async fn delete_db_instance_skip_final_snapshot(
    client: &Client,
    db_instance_id: &str,
) -> AppResult<()> {
    client
        .delete_db_instance()
        .db_instance_identifier(db_instance_id)
        .skip_final_snapshot(true)
        .send()
        .await?;
    Ok(())
}

pub async fn delete_db_instance_with_final_snapshot(
    client: &Client,
    db_instance_id: &str,
) -> AppResult<()> {
    let now = Utc::now();
    let now_formatted = now.format("%Y-%m-%d-%H-%M");
    let final_snapshot_identifier =
        sanitize_string(&format!("{}-{}", db_instance_id, now_formatted));
    client
        .delete_db_instance()
        .db_instance_identifier(db_instance_id)
        .skip_final_snapshot(false)
        .final_db_snapshot_identifier(final_snapshot_identifier)
        .send()
        .await?;
    Ok(())
}

pub async fn stop_db_instance(client: &Client, db_instance_id: &str) -> AppResult<()> {
    client
        .stop_db_instance()
        .db_instance_identifier(db_instance_id)
        .send()
        .await?;
    Ok(())
}
