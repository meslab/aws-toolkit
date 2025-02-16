use crate::AppResult;
use aws_sdk_ecs::Client;
use log::debug;

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
        for service_arn in services?.service_arns() {
            debug!("Service ARN: {:?}", service_arn);
            if service_arn.contains(cluster) {
                match client
                    .describe_services()
                    .cluster(cluster)
                    .services(service_arn)
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
                            service_arns.push(service_arn.to_owned());
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
) -> AppResult<()> {
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
pub async fn delete_service(client: &Client, cluster: &str, service_arn: &str) -> AppResult<()> {
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
        debug!("Services: {:?}", services);
        let service_arn = services
            .unwrap()
            .service_arns
            .unwrap()
            .into_iter()
            .find(|arn| arn.contains(service));
        if let Some(service_arn) = service_arn {
            debug!("Inside get_service_arn Service ARN: {:?}", service_arn);
            return Ok(service_arn);
        }
    }
    Err("Service not found".into())
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
        debug!("Tasks: {:?}", tasks);
        let task_arn = tasks.unwrap().task_arns.unwrap_or_default().pop();
        if let Some(task_arn) = task_arn {
            return Ok(task_arn);
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
    Ok(describe_tasks_result
        .tasks
        .unwrap_or_default()
        .pop()
        .unwrap()
        .container_instance_arn
        .unwrap_or_default())
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
    Ok(describe_container_instances_result
        .container_instances
        .unwrap_or_default()
        .pop()
        .unwrap()
        .ec2_instance_id
        .expect("No EC2 instance found!"))
}
