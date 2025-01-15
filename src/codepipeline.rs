use crate::AppResult;
use aws_sdk_codepipeline::error::SdkError;
use aws_sdk_codepipeline::types::StageExecutionStatus::{Failed, InProgress};
use aws_sdk_codepipeline::types::StageState;
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

pub async fn list_all_pipelines(
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

pub async fn list_pipelines(
    client: &Client,
    include: &[String],
    exclude: &[String],
) -> AppResult<Vec<String>> {
    let input = list_all_pipelines(client, include, exclude).await?;

    let state_filter =
        |x: &StageState| ![InProgress].contains(&x.latest_execution.as_ref().unwrap().status);

    list_state_pipelines_internal(client, &input, state_filter).await
}

pub async fn list_failed_pipelines(
    client: &Client,
    include: &[String],
    exclude: &[String],
) -> AppResult<Vec<String>> {
    let input = list_all_pipelines(client, include, exclude).await?;

    let state_filter = |x: &StageState| {
        let status = &x.latest_execution.as_ref().unwrap().status;
        [Failed].contains(status) && ![InProgress].contains(status)
    };

    list_state_pipelines_internal(client, &input, state_filter).await
}

async fn list_state_pipelines_internal<F>(
    client: &Client,
    input: &Vec<String>,
    filter: F,
) -> AppResult<Vec<String>>
where
    F: Fn(&StageState) -> bool + Send + Sync,
{
    let mut pipelines = Vec::new();

    for pipeline in input {
        let state = client.get_pipeline_state().name(pipeline).send().await?;
        if state
            .stage_states
            .filter(|stage_states| stage_states.iter().any(&filter))
            .is_some()
        {
            pipelines.push(pipeline.to_owned());
        }
    }

    debug!("Pipelines: {:?}", pipelines);
    Ok(pipelines)
}

pub async fn release_pipelines<'a, F, Fut>(
    client: &'a Client,
    include: &'a [String],
    exclude: &'a [String],
    pipeline_names_getter: F,
) -> AppResult<()>
where
    F: Fn(&'a Client, &'a [String], &'a [String]) -> Fut + Send + Sync,
    Fut: std::future::Future<Output = AppResult<Vec<String>>>,
{
    let pipelines = pipeline_names_getter(client, include, exclude).await?;
    for pipeline in pipelines {
        println!("{}", &pipeline);
        release_pipeline(client, &pipeline).await?;
    }
    Ok(())
}

pub async fn release_pipeline(client: &Client, pipeline_name: &str) -> AppResult<()> {
    let max_retries = 3;
    let mut retries = 0;

    sleep(Duration::from_secs(1)).await;
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
                            "ThrottlingException encountered. Retrying in {} seconds... (attempt {}/{})",
                            5 * retries, retries, max_retries
                        );
                        sleep(Duration::from_secs(5 * retries)).await;
                        continue;
                    }
                }
                return Err(SdkError::ServiceError(service_error).into());
            }
            Err(err) => return Err(err.into()),
        }
    }
}
