use aws_config::default_provider::credentials::DefaultCredentialsChain;
use aws_sdk_ecs::config::Region;
use aws_sdk_ecs::{Client, Config};
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

pub async fn get_service_arns(
    client: &Client,
    cluster: &str,
    desired_count: i32,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut service_arns: Vec<String> = Vec::new();
    let mut services_stream = client
        .list_services()
        .cluster(cluster)
        .max_results(100)
        .into_paginator()
        .send();

    while let Some(services) = services_stream.next().await {
        debug!("Services: {:?}", services);
        for service_arn in services.unwrap().service_arns.unwrap() {
            debug!("Service ARN: {:?}", service_arn);
            if service_arn.contains(cluster) {
                debug!("Service ARN: {}", service_arn);
                match client
                    .describe_services()
                    .cluster(cluster)
                    .services(&service_arn)
                    .send()
                    .await
                {
                    Ok(service) => {
                        debug!("Service: {:?}", service);
                        if service
                            .services
                            .unwrap()
                            .first()
                            .unwrap()
                            .desired_count
                            .gt(&desired_count)
                        {
                            service_arns.push(service_arn);
                        }
                    }
                    Err(e) => {
                        debug!("Error: {:?}", e);
                    }
                }
            }
        }
    }
    Ok(service_arns)
}

#[async_recursion::async_recursion]
pub async fn scale_down_service(
    client: &Client,
    cluster: &str,
    service_arn: &str,
    desired_count: i32,
) -> Result<(), Box<dyn std::error::Error>> {
    match client
        .update_service()
        .cluster(cluster)
        .service(service_arn)
        .desired_count(desired_count)
        .send()
        .await
    {
        Ok(_) => Ok(()),
        _ => {
            tokio::time::sleep(std::time::Duration::from_secs(15)).await;
            scale_down_service(client, cluster, service_arn, desired_count).await?;
            Ok(())
        }
    }
}

#[async_recursion::async_recursion]
pub async fn delete_service(
    client: &Client,
    cluster: &str,
    service_arn: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    match client
        .delete_service()
        .cluster(cluster)
        .service(service_arn)
        .send()
        .await
    {
        Ok(_) => Ok(()),
        _ => {
            tokio::time::sleep(std::time::Duration::from_secs(15)).await;
            delete_service(client, cluster, service_arn).await?;
            Ok(())
        }
    }
}
