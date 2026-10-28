use crate::AppResult;
use aws_sdk_ec2::Client;
use aws_sdk_ec2::error::{ProvideErrorMetadata, SdkError};
use aws_sdk_ec2::types::Filter;
use aws_sdk_ec2::types::InstanceStateName::{ShuttingDown, Terminated};
use aws_sdk_ec2::types::NatGatewayState::{Deleted, Deleting};
use log::debug;
use std::time::Duration;

const MAX_RETRIES: u32 = 5;
const RETRY_DELAY: Duration = Duration::from_secs(15);

fn is_not_found<E, R>(error: &SdkError<E, R>) -> bool
where
    E: ProvideErrorMetadata,
{
    matches!(
        error.as_service_error().and_then(|e| e.code()),
        Some("InvalidInstanceID.NotFound") | Some("NatGatewayNotFound")
    )
}

pub async fn get_nat_gateway_ids(
    client: &Client,
    cluster: &str,
    tag_key: &str,
) -> AppResult<Vec<String>> {
    let mut nat_gateway_ids: Vec<String> = Vec::new();
    let filter = Filter::builder()
        .name(format!("tag:{}", tag_key))
        .values(cluster)
        .build();

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

                    let has_cluster_tag = nat_gateway.tags().iter().any(|tag| {
                        tag.key() == Some(tag_key)
                            && tag.value().is_some_and(|value| value.contains(cluster))
                    });

                    if has_cluster_tag {
                        nat_gateway.nat_gateway_id().map(ToOwned::to_owned)
                    } else {
                        None
                    }
                }),
        );
    }
    Ok(nat_gateway_ids)
}

pub async fn delete_nat_gateway(client: &Client, gateway_id: &str) -> AppResult<()> {
    let mut attempt = 0;

    loop {
        match client
            .delete_nat_gateway()
            .nat_gateway_id(gateway_id)
            .send()
            .await
        {
            Ok(_) => return Ok(()),
            Err(error) => {
                if is_not_found(&error) {
                    debug!("NAT gateway {} is already gone: {}", gateway_id, error);
                    return Ok(());
                }

                if attempt >= MAX_RETRIES {
                    return Err(error.into());
                }

                attempt += 1;
                debug!(
                    "Retrying deletion of NAT gateway {} ({}/{}): {}",
                    gateway_id, attempt, MAX_RETRIES, error
                );
                tokio::time::sleep(RETRY_DELAY).await;
            }
        }
    }
}

pub async fn get_ec2_instances_ids(
    client: &Client,
    cluster: &str,
    tag_key: &str,
) -> AppResult<Vec<String>> {
    let mut ec2_instances_ids: Vec<String> = Vec::new();
    let prefix_filter = Filter::builder()
        .name(format!("tag:{}", tag_key))
        .values(cluster)
        .build();

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
                let state = instance.state().and_then(|state| state.name());
                if state.is_some_and(|state| [ShuttingDown, Terminated].contains(state)) {
                    continue;
                }

                let has_cluster_tag = instance.tags().iter().any(|tag| {
                    tag.key() == Some(tag_key)
                        && tag.value().is_some_and(|value| value.contains(cluster))
                });

                if has_cluster_tag && let Some(instance_id) = instance.instance_id() {
                    ec2_instances_ids.push(instance_id.to_owned());
                }
            }
        }
    }
    Ok(ec2_instances_ids)
}

pub async fn terminate_ec2_instance(client: &Client, instance_id: &str) -> AppResult<()> {
    let mut attempt = 0;

    loop {
        match client
            .terminate_instances()
            .instance_ids(instance_id)
            .send()
            .await
        {
            Ok(_) => return Ok(()),
            Err(error) => {
                if is_not_found(&error) {
                    debug!("EC2 instance {} is already gone: {}", instance_id, error);
                    return Ok(());
                }

                if attempt >= MAX_RETRIES {
                    return Err(error.into());
                }

                attempt += 1;
                debug!(
                    "Retrying termination of EC2 instance {} ({}/{}): {}",
                    instance_id, attempt, MAX_RETRIES, error
                );
                tokio::time::sleep(RETRY_DELAY).await;
            }
        }
    }
}
