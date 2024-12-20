use aws_config::default_provider::credentials::DefaultCredentialsChain;
use aws_config::Region;
use aws_sdk_ecs::{Client, Config};
use log::debug;

pub async fn initialize_client(region: Region, profile: &str) -> Client {
    let credentials_provider = DefaultCredentialsChain::builder()
        .profile_name(profile)
        .build()
        .await;
    let config = Config::builder()
        .credentials_provider(credentials_provider)
        .region(region)
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

pub async fn get_service_arn(
    ecs_client: &Client,
    cluster: &str,
    service: &str,
) -> Result<String, Box<dyn std::error::Error>> {
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

pub async fn get_task_arn(
    ecs_client: &Client,
    cluster: &str,
    service: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut ecs_tasks_stream = ecs_client
        .list_tasks()
        .cluster(cluster)
        .max_results(100)
        .service_name(service)
        .into_paginator()
        .send();
    while let Some(tasks) = ecs_tasks_stream.next().await {
        debug!("Tasks: {:?}", tasks);
        let task_arn = tasks
        .unwrap()
        .task_arns
        .unwrap_or_default()
        .pop();
        if let Some(task_arn) = task_arn {
            return Ok(task_arn)
        }
     }
    Err("Task not found".into())
}

pub async fn get_task_container_arn(
    ecs_client: &Client,
    cluster: &str,
    task_arn: &str,
) -> Result<String, Box<dyn std::error::Error>> {
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
) -> Result<String, Box<dyn std::error::Error>> {
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
