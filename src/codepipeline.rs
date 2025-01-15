use crate::AppResult;
use aws_sdk_codepipeline::error::SdkError;
use aws_sdk_codepipeline::types::StageExecutionStatus::{Failed, InProgress};
use aws_sdk_codepipeline::Client;
use log::debug;
use std::time::Duration;
use tokio::time::sleep;

async fn list_filtered_pipelines_internal<F>(client: &Client, filter: F) -> AppResult<Vec<String>>
where
    F: Fn(&str) -> bool + Send + Sync,
{
    let mut pipelines = Vec::new();

    let mut pipelines_stream = client.list_pipelines().into_paginator().send();

    while let Some(output) = pipelines_stream.next().await {
        for pipeline in output?.pipelines.unwrap_or_default() {
            if let Some(pipeline_name) = pipeline.name {
                if filter(&pipeline_name) {
                    pipelines.push(pipeline_name);
                }
            }
        }
    }

    debug!("Pipelines: {:?}", pipelines);
    Ok(pipelines)
}

pub async fn list_pipelines(
    client: &Client,
    include: &[String],
    exclude: &[String],
) -> AppResult<Vec<String>> {
    let include: Vec<_> = include.iter().map(|x| x.as_str()).collect();
    let exclude: Vec<_> = exclude.iter().map(|x| x.as_str()).collect();

    let filter = |pipeline_name: &str| {
        include.iter().any(|&x| pipeline_name.contains(x))
            && exclude.iter().all(|&x| !pipeline_name.contains(x))
    };

    list_filtered_pipelines_internal(client, filter).await
}
pub async fn list_failed_pipelines(
    client: &Client,
    include: &[String],
    exclude: &[String],
) -> AppResult<Vec<String>> {
    let include: Vec<_> = include.iter().map(|x| x.as_str()).collect();
    let exclude: Vec<_> = exclude.iter().map(|x| x.as_str()).collect();

    let filter = |pipeline_name: &str| {
        include.iter().any(|&x| pipeline_name.contains(x))
            && exclude.iter().all(|&x| !pipeline_name.contains(x))
    };

    let input = list_filtered_pipelines_internal(client, filter).await?;
    list_failed_pipelines_internal(client, &input).await
}

async fn list_failed_pipelines_internal(
    client: &Client,
    input: &Vec<String>,
) -> AppResult<Vec<String>> {
    let mut pipelines = Vec::new();

    for pipeline in input {
        let state = client.get_pipeline_state().name(pipeline).send().await?;
        if state
            .stage_states
            .filter(|stage_states| {
                stage_states.iter().any(|x| {
                    [Failed, InProgress].contains(&x.latest_execution.as_ref().unwrap().status)
                })
            })
            .is_some()
        {
            pipelines.push(pipeline.to_owned());
        }
    }

    debug!("Pipelines: {:?}", pipelines);
    Ok(pipelines)
}

pub async fn release_pipeline(client: &Client, pipeline_name: &str) -> AppResult<()> {
    let max_retries = 3;
    let mut retries = 0;

    loop {
        match client
            .start_pipeline_execution()
            .name(pipeline_name)
            .send()
            .await
        {
            Ok(_) => return Ok(()), // Success, exit the loop
            Err(SdkError::ServiceError(service_error)) => {
                if let Some(code) = service_error.err().meta().code() {
                    if code == "ThrottlingException" && retries < max_retries {
                        retries += 1;
                        eprintln!(
                            "ThrottlingException encountered. Retrying in 5 seconds... (attempt {}/{})",
                            retries, max_retries
                        );
                        sleep(Duration::from_secs(5)).await;
                        continue;
                    }
                }
                return Err(SdkError::ServiceError(service_error).into());
            }
            Err(err) => return Err(err.into()), // Propagate non-service errors
        }
    }
}
