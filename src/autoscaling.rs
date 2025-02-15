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

    while let Some(asg_result) = asg_stream.next().await {
        //let asg_result = asg_result?;
        debug!("ASG: {:?}", asg_result);
        asgs.extend(asg_result?.auto_scaling_groups().iter().filter_map(|asg| {
            let asg_name = asg.auto_scaling_group_name.as_deref()?;
            if !asg_name.contains(cluster) {
                return None;
            }
            if asg
                .desired_capacity
                .expect("Cannnot get group desired capacity.")
                .gt(&desired_capacity)
            {
                return None;
            }
            Some(asg_name.to_owned())
        }));
    }
    Ok(asgs)
}

pub async fn scale_down_asg(
    client: &Client,
    asg_name: &str,
    desired_capacity: i32,
) -> AppResult<()> {
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
