use crate::AppResult;
use aws_sdk_codepipeline::Client;
use aws_sdk_codepipeline::error::SdkError;
use aws_sdk_codepipeline::types::StageExecutionStatus::{Failed, InProgress, Succeeded};
use aws_sdk_codepipeline::types::StageState;
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
        pipelines.extend(output?.pipelines().iter().filter_map(|p| {
            let pipeline_name = p.name()?;
            if filter(pipeline_name) {
                Some(pipeline_name.to_owned())
            } else {
                None
            }
        }));
    }

    debug!("Pipelines: {:?}", pipelines);
    Ok(pipelines)
}

pub async fn list_all_pipelines(
    client: &Client,
    include: &[String],
    exclude: &[String],
) -> AppResult<Vec<String>> {
    let include = include.to_vec();
    let exclude = exclude.to_vec();

    let filter = |pipeline_name: &str| {
        include.iter().any(|x| pipeline_name.contains(x))
            && exclude.iter().all(|x| !pipeline_name.contains(x))
    };

    list_filtered_pipelines_internal(client, filter).await
}

pub async fn list_pipelines(
    client: &Client,
    include: &[String],
    exclude: &[String],
) -> AppResult<Vec<String>> {
    let input = list_all_pipelines(client, include, exclude).await?;

    let filter_failure = |x: &StageState| {
        if x.latest_execution.is_none() {
            return false;
        }
        let status = get_stage_status(x);
        [Succeeded, Failed].contains(status)
    };

    let filter_progress = |x: &StageState| {
        if x.latest_execution.is_none() {
            return true;
        }
        let status = get_stage_status(x);
        ![InProgress].contains(status)
    };

    list_state_pipelines_internal(client, &input, filter_failure, filter_progress).await
}

pub async fn list_failed_pipelines(
    client: &Client,
    include: &[String],
    exclude: &[String],
) -> AppResult<Vec<String>> {
    let input = list_all_pipelines(client, include, exclude).await?;

    let filter_failure = |x: &StageState| {
        if x.latest_execution.is_none() {
            return false;
        }
        let status = get_stage_status(x);
        [Failed].contains(status)
    };

    let filter_progress = |x: &StageState| {
        if x.latest_execution.is_none() {
            return true;
        }
        let status = get_stage_status(x);
        ![InProgress].contains(status)
    };

    list_state_pipelines_internal(client, &input, filter_failure, filter_progress).await
}

fn get_stage_status(
    stage_state: &StageState,
) -> &aws_sdk_codepipeline::types::StageExecutionStatus {
    let status = &stage_state
        .latest_execution()
        .unwrap_or_else(|| {
            panic!(
                "Cannot extract status from the latest execution of {}.",
                &stage_state.stage_name().unwrap_or_default()
            )
        })
        .status;
    status
}

async fn list_state_pipelines_internal<Ff, Fp>(
    client: &Client,
    input: &Vec<String>,
    filter_failure: Ff,
    filter_progress: Fp,
) -> AppResult<Vec<String>>
where
    Ff: Fn(&StageState) -> bool + Send + Sync,
    Fp: Fn(&StageState) -> bool + Send + Sync,
{
    let mut pipelines = Vec::new();

    for pipeline in input {
        let state = client.get_pipeline_state().name(pipeline).send().await?;
        if state
            .stage_states
            .filter(|stage_states| stage_states.iter().any(&filter_failure))
            .filter(|stage_states| stage_states.iter().all(&filter_progress))
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
            Ok(_) => {
                sleep(Duration::from_secs(1)).await;
                return Ok(());
            } // Success, exit the loop
            Err(SdkError::ServiceError(service_error)) => {
                if let Some(code) = service_error.err().meta().code() {
                    if code == "ThrottlingException" && retries < max_retries {
                        retries += 1;
                        eprintln!(
                            "ThrottlingException encountered. Retrying in {} seconds... (attempt {}/{})",
                            25 * retries,
                            retries,
                            max_retries
                        );
                        sleep(Duration::from_secs(20 * retries)).await;
                        continue;
                    }
                }
                return Err(SdkError::ServiceError(service_error).into());
            }
            Err(err) => return Err(err.into()),
        }
    }
}
