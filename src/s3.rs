use crate::AppResult;
use crate::utils::matches_filters;
use aws_sdk_s3::Client as S3Client;
use aws_sdk_s3::error::SdkError;
use aws_sdk_s3::types::MetadataDirective;
use log::debug;
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use tokio::task::JoinSet;

const MAX_CONCURRENT_COPIES: usize = 8;

pub async fn save_bucket_policy(client: &S3Client, bucket: &str) -> AppResult<Option<String>> {
    // Save current bucket policy JSON
    match client.get_bucket_policy().bucket(bucket).send().await {
        Ok(resp) => Ok(resp.policy),
        Err(err) => {
            let policy_missing = matches!(
                &err,
                SdkError::ServiceError(context)
                    if context.err().meta().code() == Some("NoSuchBucketPolicy")
            );

            if policy_missing {
                Ok(None)
            } else {
                Err(err.into())
            }
        }
    }
}

pub async fn restore_bucket_policy(client: &S3Client, bucket: &str, policy: &str) -> AppResult<()> {
    client
        .put_bucket_policy()
        .bucket(bucket)
        .policy(policy)
        .send()
        .await?;

    Ok(())
}

async fn list_filtered_buckets_internal<F>(client: &S3Client, filter: F) -> AppResult<Vec<String>>
where
    F: Fn(&str) -> bool + Send + Sync,
{
    let mut bucket_names: Vec<String> = Vec::new();
    let mut buckets_stream = client
        .list_buckets()
        .max_buckets(100)
        .into_paginator()
        .send();

    while let Some(buckets) = buckets_stream.next().await {
        debug!("Buckets: {:?}", buckets);

        bucket_names.extend(buckets?.buckets().iter().filter_map(|b| {
            let bucket_name = b.name()?;
            if filter(bucket_name) {
                Some(bucket_name.to_owned())
            } else {
                None
            }
        }));
    }

    Ok(bucket_names)
}

pub async fn get_buckets(
    client: &S3Client,
    include: &[String],
    exclude: &[String],
) -> AppResult<Vec<String>> {
    let include = include.to_vec();
    let exclude = exclude.to_vec();

    let filter = |name: &str| matches_filters(name, &include, &exclude);

    list_filtered_buckets_internal(client, filter).await
}

async fn needs_rescan(client: &S3Client, bucket: &str, key: &str) -> Result<bool, String> {
    let tags = client
        .get_object_tagging()
        .bucket(bucket)
        .key(key)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    Ok(!tags
        .tag_set()
        .iter()
        .any(|tag| tag.key() == "GuardDutyMalwareScanStatus"))
}

pub async fn copy_all_objects(client: &S3Client, bucket: &str, dry_run: bool) -> AppResult<usize> {
    let paginator = client
        .list_objects_v2()
        .bucket(bucket)
        .into_paginator()
        .send();

    tokio::pin!(paginator);

    let mut tasks: JoinSet<Result<bool, (String, String)>> = JoinSet::new();
    let mut counter = 0usize;
    let mut counter_total = 0usize;
    let mut failures = 0usize;

    while let Some(page) = paginator.next().await {
        let page = page?;

        if let Some(contents) = page.contents {
            for object in contents {
                let Some(key) = object.key else {
                    continue;
                };

                counter_total += 1;

                let client = client.clone();
                let bucket = bucket.to_owned();
                let task_key = key.clone();
                tasks.spawn(async move {
                    let result = async {
                        let needs_copy = needs_rescan(&client, &bucket, &task_key).await?;
                        if needs_copy && !dry_run {
                            copy_to_self(&client, &bucket, &task_key)
                                .await
                                .map_err(|e| e.to_string())?;
                        }
                        Ok(needs_copy)
                    }
                    .await;

                    result.map_err(|e: String| (task_key, e))
                });

                if tasks.len() >= MAX_CONCURRENT_COPIES
                    && let Some(result) = tasks.join_next().await
                {
                    handle_copy_result(result, &mut counter, &mut failures);
                }

                if counter_total.is_multiple_of(100) {
                    println!("processed {} objects, copied {}", counter_total, counter);
                }
            }
        }
    }

    while let Some(result) = tasks.join_next().await {
        handle_copy_result(result, &mut counter, &mut failures);
    }

    if failures > 0 {
        eprintln!("{} objects failed to copy", failures);
    }

    Ok(counter)
}

fn handle_copy_result(
    result: Result<Result<bool, (String, String)>, tokio::task::JoinError>,
    counter: &mut usize,
    failures: &mut usize,
) {
    match result {
        Ok(Ok(true)) => *counter += 1,
        Ok(Ok(false)) => {}
        Ok(Err((key, error))) => {
            *failures += 1;
            eprintln!(
                "Error: file '{}' could not be copied. Details: {}",
                key, error
            );
        }
        Err(error) => {
            *failures += 1;
            eprintln!("Error: copy task failed. Details: {}", error);
        }
    }
}

pub async fn copy_to_self(client: &S3Client, bucket: &str, key: &str) -> AppResult<()> {
    let encoded_key = utf8_percent_encode(key, NON_ALPHANUMERIC).to_string();
    let copy_source = format!("{}/{}", bucket, encoded_key);

    client
        .copy_object()
        .bucket(bucket)
        .key(key)
        .copy_source(copy_source)
        .metadata_directive(MetadataDirective::Copy)
        .send()
        .await?;

    Ok(())
}
