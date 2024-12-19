use aws_config::default_provider::credentials::DefaultCredentialsChain;
use aws_sdk_elasticloadbalancingv2::config::Region;
use aws_sdk_elasticloadbalancingv2::{Client, Config};
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

pub async fn list_load_balancers(
    client: &Client,
    cluster: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut load_balancers = Vec::new();
    let mut load_balancers_stream = client.describe_load_balancers().into_paginator().send();

    while let Some(load_balancer) = load_balancers_stream.next().await {
        debug!("Load Balancers: {:?}", load_balancer);
        for lb in load_balancer.unwrap().load_balancers.unwrap() {
            if lb.load_balancer_name.unwrap().contains(cluster) {
                load_balancers.push(lb.load_balancer_arn.unwrap());
            }
        }
    }
    Ok(load_balancers)
}

pub async fn delete_load_balancer(
    client: &Client,
    load_balancer_arn: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    client
        .delete_load_balancer()
        .load_balancer_arn(load_balancer_arn)
        .send()
        .await?;
    Ok(())
}
