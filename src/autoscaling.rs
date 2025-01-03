use crate::AppResult;
use aws_sdk_autoscaling::Client;
use log::debug;

pub async fn list_asgs(
    client: &Client,
    cluster: &str,
    desired_capacity: i32,
) -> AppResult<Vec<String>> {
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
                && group.desired_capacity.unwrap().gt(&desired_capacity)
            {
                asgs.push(group.auto_scaling_group_name.unwrap());
            }
        }
    }
    Ok(asgs)
}

pub async fn scale_down_asg(client: &Client, asg_name: &str, desired_capacity: i32) -> AppResult<()> {
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
