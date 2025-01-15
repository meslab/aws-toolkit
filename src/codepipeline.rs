use crate::AppResult;
use aws_sdk_codepipeline::Client;
use log::debug;

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
