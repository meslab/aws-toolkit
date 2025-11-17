use crate::AppResult;
use aws_sdk_ec2::Client;
use aws_sdk_ec2::types::Filter;
use aws_sdk_ec2::types::InstanceStateName::{ShuttingDown, Terminated};
use aws_sdk_ec2::types::NatGatewayState::{Deleted, Deleting};

pub async fn get_nat_gateway_ids(client: &Client, cluster: &str) -> AppResult<Vec<String>> {
    let mut nat_gateway_ids: Vec<String> = Vec::new();
    let filter = Filter::builder().name("tag:Prefix").values(cluster).build();

    let mut nat_gateway_stream = client
        .describe_nat_gateways()
        .filter(filter)
        .max_results(100)
        .into_paginator()
        .send();

    while let Some(nat_gateways) = nat_gateway_stream.next().await {
        nat_gateway_ids.extend(
            nat_gateways?
                .nat_gateways()
                .iter()
                .filter_map(|nat_gateway| {
                    if [Deleted, Deleting].contains(nat_gateway.state()?) {
                        return None;
                    }

                    let get_id_string = || nat_gateway.nat_gateway_id().map(ToOwned::to_owned);

                    nat_gateway
                        .tags()
                        .iter()
                        .find_map(|tag| tag.value().filter(|v| v.contains(cluster)))
                        .and_then(|_| get_id_string())
                }),
        );
    }
    Ok(nat_gateway_ids)
}

#[async_recursion::async_recursion]
pub async fn delete_nat_gateway(client: &Client, gateway_id: &str) -> AppResult<()> {
    match client
        .delete_nat_gateway()
        .nat_gateway_id(gateway_id)
        .send()
        .await
    {
        Ok(_) => Ok(()),
        _ => {
            tokio::time::sleep(std::time::Duration::from_secs(15)).await;
            delete_nat_gateway(client, gateway_id).await?;
            Ok(())
        }
    }
}

pub async fn get_ec2_instances_ids(client: &Client, cluster: &str) -> AppResult<Vec<String>> {
    let mut ec2_instances_ids: Vec<String> = Vec::new();
    let prefix_filter = Filter::builder().name("tag:Prefix").values(cluster).build();

    let state_filter = Filter::builder()
        .name("instance-state-name")
        .values("running")
        .build();

    let mut ec2_instances_stream = client
        .describe_instances()
        .filters(prefix_filter)
        .filters(state_filter)
        .max_results(100)
        .into_paginator()
        .send();

    while let Some(ec2_instances) = ec2_instances_stream.next().await {
        for reservation in ec2_instances?.reservations() {
            for instance in reservation.instances() {
                if instance.tags().iter().any(|t| {
                    t.value()
                        .expect("Cannot extract tag value.")
                        .contains(cluster)
                }) && ![ShuttingDown, Terminated].contains(
                    instance
                        .state()
                        .expect("Cannot extract ec2 instance state.")
                        .name()
                        .expect("Cannot extract ec2 instance state name."),
                ) {
                    ec2_instances_ids.push(
                        instance
                            .instance_id()
                            .expect("Cannot extract EC2 Instance ID.")
                            .to_owned(),
                    );
                }
            }
        }
    }
    Ok(ec2_instances_ids)
}

#[async_recursion::async_recursion]
pub async fn terminate_ec2_instance(client: &Client, instance_id: &str) -> AppResult<()> {
    match client
        .terminate_instances()
        .instance_ids(instance_id)
        .send()
        .await
    {
        Ok(_) => Ok(()),
        _ => {
            tokio::time::sleep(std::time::Duration::from_secs(15)).await;
            terminate_ec2_instance(client, instance_id).await?;
            Ok(())
        }
    }
}
