use aws_config::default_provider::credentials::DefaultCredentialsChain;
use aws_sdk_autoscaling::config::Region;
use aws_sdk_autoscaling::{Client, Config};
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

pub async fn list_asgs(
    client: &Client,
    cluster: &str,
    desired_capacity: i32,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut asgs = Vec::new();
    let mut asg_stream = client
        .describe_auto_scaling_groups()
        .max_records(100)
        .into_paginator()
        .send();

    while let Some(asg) = asg_stream.next().await {
        debug!("ASG: {:?}", asg);
        for group in asg.unwrap().auto_scaling_groups.unwrap() {
            if group
                .auto_scaling_group_name
                .as_ref()
                .unwrap()
                .contains(cluster)
                && group
                    .desired_capacity
                    .unwrap()
                    .gt(&desired_capacity)
            {
                asgs.push(group.auto_scaling_group_name.unwrap());
            }
        }
    }
    Ok(asgs)
}

pub async fn scale_down_asg(
    client: &Client,
    asg_name: &str,
    desired_capacity: i32,
) -> Result<(), Box<dyn std::error::Error>> {
    client
        .update_auto_scaling_group()
        .auto_scaling_group_name(asg_name)
        .desired_capacity(desired_capacity)
        .min_size(desired_capacity)
        .max_size(desired_capacity)
        .send()
        .await?;
    Ok(())
}
