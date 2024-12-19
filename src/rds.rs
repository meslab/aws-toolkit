use aws_config::default_provider::credentials::DefaultCredentialsChain;
use aws_sdk_rds::config::Region;
use aws_sdk_rds::{Client, Config};
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

pub async fn list_db_instances(
    client: &Client,
    cluster: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
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
                    || instance
                        .db_instance_status
                        .unwrap()
                        .contains("stopped"))
            {
                db_instances.push(instance.db_instance_identifier.unwrap());
            }
        }
    }
    Ok(db_instances)
}

pub async fn disable_deletion_protection(
    client: &Client,
    db_instance_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    client
        .modify_db_instance()
        .db_instance_identifier(db_instance_id)
        .set_deletion_protection(Some(false))
        .apply_immediately(true)
        .send()
        .await?;
    Ok(())
}

pub async fn delete_db_instance(
    client: &Client,
    db_instance_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    client
        .delete_db_instance()
        .db_instance_identifier(db_instance_id)
        .skip_final_snapshot(true)
        .send()
        .await?;
    Ok(())
}

pub async fn stop_db_instance(
    client: &Client,
    db_instance_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    client
        .stop_db_instance()
        .db_instance_identifier(db_instance_id)
        .db_instance_identifier(db_instance_id)
        .send()
        .await?;
    Ok(())
}
