use crate::AppResult;
use aws_sdk_ecs::Client;
use aws_sdk_ecs::error::{ProvideErrorMetadata, SdkError};
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
        Some("ServiceNotFoundException")
    )
}

pub async fn get_service_arns(
    client: &Client,
    cluster: &str,
    desired_count: i32,
) -> AppResult<Vec<String>> {
    let mut service_arns: Vec<String> = Vec::new();
    let mut services_stream = client
        .list_services()
        .cluster(cluster)
        .max_results(100)
        .into_paginator()
        .send();

    while let Some(services) = services_stream.next().await {
        debug!("Services: {:?}", services);
        let relevant_arns: Vec<String> = services?
            .service_arns()
            .iter()
            .filter(|arn| arn.contains(cluster))
            .map(|s| s.to_owned())
            .collect();

        for relevant_arns_chunk in relevant_arns.chunks(10) {
            if relevant_arns_chunk.is_empty() {
                continue;
            }

            let services = client
                .describe_services()
                .cluster(cluster)
                .set_services(Some(relevant_arns_chunk.to_vec()))
                .send()
                .await?;

            service_arns.extend(
                services
                    .services()
                    .iter()
                    .filter(|service| service.desired_count > desired_count)
                    .filter_map(|service| service.service_arn().map(ToOwned::to_owned)),
            );
        }
    }
    Ok(service_arns)
}

pub async fn scale_down_service(
    client: &Client,
    cluster: &str,
    service_arn: &str,
    desired_count: i32,
) -> AppResult<()> {
    let mut attempt = 0;

    loop {
        match client
            .update_service()
            .cluster(cluster)
            .service(service_arn)
            .desired_count(desired_count)
            .send()
            .await
        {
            Ok(_) => return Ok(()),
            Err(error) => {
                if is_not_found(&error) {
                    debug!("Service {} is already gone: {}", service_arn, error);
                    return Ok(());
                }

                if attempt >= MAX_RETRIES {
                    return Err(error.into());
                }

                attempt += 1;
                debug!(
                    "Retrying scale down of {} ({}/{}): {}",
                    service_arn, attempt, MAX_RETRIES, error
                );
                tokio::time::sleep(RETRY_DELAY).await;
            }
        }
    }
}

pub async fn delete_service(client: &Client, cluster: &str, service_arn: &str) -> AppResult<()> {
    let mut attempt = 0;

    loop {
        match client
            .delete_service()
            .cluster(cluster)
            .service(service_arn)
            .send()
            .await
        {
            Ok(_) => return Ok(()),
            Err(error) => {
                if is_not_found(&error) {
                    debug!("Service {} is already gone: {}", service_arn, error);
                    return Ok(());
                }

                if attempt >= MAX_RETRIES {
                    return Err(error.into());
                }

                attempt += 1;
                debug!(
                    "Retrying deletion of {} ({}/{}): {}",
                    service_arn, attempt, MAX_RETRIES, error
                );
                tokio::time::sleep(RETRY_DELAY).await;
            }
        }
    }
}

pub async fn get_service_arn(
    ecs_client: &Client,
    cluster: &str,
    service: &str,
) -> AppResult<String> {
    let mut ecs_services_stream = ecs_client
        .list_services()
        .cluster(cluster)
        .max_results(100)
        .into_paginator()
        .send();

    while let Some(services) = ecs_services_stream.next().await {
        let services = services?;
        debug!("Services: {:?}", services);
        if let Some(service_arn) = services
            .service_arns()
            .iter()
            .find(|arn| arn.contains(service))
        {
            debug!("Inside get_service_arn Service ARN: {:?}", service_arn);
            return Ok(service_arn.to_owned());
        }
    }
    Err(format!("Service '{}' not found in cluster '{}'", service, cluster).into())
}

pub async fn get_task_arn(ecs_client: &Client, cluster: &str, service: &str) -> AppResult<String> {
    let mut ecs_tasks_stream = ecs_client
        .list_tasks()
        .cluster(cluster)
        .max_results(100)
        .service_name(service)
        .into_paginator()
        .send();
    while let Some(tasks) = ecs_tasks_stream.next().await {
        let tasks = tasks?;
        debug!("Tasks: {:?}", tasks);
        if let Some(task_arn) = tasks.task_arns().last() {
            return Ok(task_arn.to_owned());
        }
    }
    Err("Task not found".into())
}

pub async fn get_task_container_arn(
    ecs_client: &Client,
    cluster: &str,
    task_arn: &str,
) -> AppResult<String> {
    let describe_tasks_result = ecs_client
        .describe_tasks()
        .cluster(cluster)
        .tasks(task_arn)
        .send()
        .await?;
    let task = describe_tasks_result
        .tasks
        .unwrap_or_default()
        .pop()
        .ok_or("Task not found")?;

    task.container_instance_arn
        .ok_or_else(|| "Container instance ARN not found for task".into())
}

pub async fn get_container_arn(
    ecs_client: &Client,
    cluster: &str,
    container_instance_arn: &str,
) -> AppResult<String> {
    let describe_container_instances_result = ecs_client
        .describe_container_instances()
        .cluster(cluster)
        .container_instances(container_instance_arn)
        .send()
        .await?;

    let container_instance = describe_container_instances_result
        .container_instances
        .unwrap_or_default()
        .pop()
        .ok_or("Container instance not found")?;

    container_instance
        .ec2_instance_id
        .ok_or_else(|| "No EC2 instance found for container instance".into())
}
