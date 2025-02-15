use crate::AppResult;
use aws_sdk_elasticloadbalancingv2::Client;
use log::debug;

pub async fn list_load_balancers(client: &Client, cluster: &str) -> AppResult<Vec<String>> {
    let mut load_balancers = Vec::new();
    let mut load_balancers_stream = client.describe_load_balancers().into_paginator().send();

    while let Some(load_balancer_output) = load_balancers_stream.next().await {
        debug!("Load Balancers: {:?}", load_balancer_output);
        load_balancers.extend(load_balancer_output?.load_balancers().iter().filter_map(
            |load_balancer| {
                if !load_balancer
                    .load_balancer_name
                    .as_deref()?
                    .contains(cluster)
                {
                    return None;
                }
                Some(load_balancer.load_balancer_arn.as_deref()?.to_owned())
            },
        ));
    }
    Ok(load_balancers)
}

pub async fn delete_load_balancer(client: &Client, load_balancer_arn: &str) -> AppResult<()> {
    client
        .delete_load_balancer()
        .load_balancer_arn(load_balancer_arn)
        .send()
        .await?;
    Ok(())
}
