use crate::Result;
use aws_sdk_rds::Client;
use log::debug;

pub async fn list_db_instances(client: &Client, cluster: &str) -> Result<Vec<String>> {
    let mut db_instances = Vec::new();
    let mut db_instances_stream = client
        .describe_db_instances()
        .max_records(100)
        .into_paginator()
        .send();

    while let Some(db_instance) = db_instances_stream.next().await {
        debug!("DB Instances: {:?}", db_instance);
        for instance in db_instance.unwrap().db_instances.unwrap() {
            if instance
                .db_instance_identifier
                .as_ref()
                .unwrap()
                .contains(cluster)
                && (instance
                    .db_instance_status
                    .as_ref()
                    .unwrap()
                    .contains("available")
                    || instance.db_instance_status.unwrap().contains("stopped"))
            {
                db_instances.push(instance.db_instance_identifier.unwrap());
            }
        }
    }
    Ok(db_instances)
}

pub async fn disable_deletion_protection(client: &Client, db_instance_id: &str) -> Result<()> {
    client
        .modify_db_instance()
        .db_instance_identifier(db_instance_id)
        .set_deletion_protection(Some(false))
        .apply_immediately(true)
        .send()
        .await?;
    Ok(())
}

pub async fn delete_db_instance(client: &Client, db_instance_id: &str) -> Result<()> {
    client
        .delete_db_instance()
        .db_instance_identifier(db_instance_id)
        .skip_final_snapshot(true)
        .send()
        .await?;
    Ok(())
}

pub async fn stop_db_instance(client: &Client, db_instance_id: &str) -> Result<()> {
    client
        .stop_db_instance()
        .db_instance_identifier(db_instance_id)
        .db_instance_identifier(db_instance_id)
        .send()
        .await?;
    Ok(())
}
