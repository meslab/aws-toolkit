use crate::AppResult;
use aws_sdk_s3::Client as S3Client;
use aws_sdk_s3::types::MetadataDirective;
use log::debug;
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};

pub async fn save_bucket_policy(client: &S3Client, bucket: &str) -> AppResult<Option<String>> {
    // Save current bucket policy JSON
    match client.get_bucket_policy().bucket(bucket).send().await {
        Ok(resp) => Ok(resp.policy),
        Err(_) => Ok(None),
    }
}

pub async fn restore_bucket_policy(client: &S3Client, bucket: &str, policy: &str) -> AppResult<()> {
    client.delete_bucket_policy().bucket(bucket).send().await?;

    if let Err(e) = client
        .put_bucket_policy()
        .bucket(bucket)
        .policy(policy)
        .send()
        .await
    {
        eprintln!("Failure restore {} bucket policy {}.", bucket, e);
    };

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

    let filter = |name_name: &str| {
        include.iter().any(|x| name_name.contains(x))
            && exclude.iter().all(|x| !name_name.contains(x))
    };

    list_filtered_buckets_internal(client, filter).await
}

pub async fn copy_all_objects(client: &S3Client, bucket: &str) -> Result<usize, aws_sdk_s3::Error> {
    let paginator = client
        .list_objects_v2()
        .bucket(bucket)
        .into_paginator()
        .send();

    tokio::pin!(paginator);

    let mut counter = 0;
    let mut counter_total = 0;
    while let Some(page) = paginator.next().await {
        let page = page?;

        if let Some(contents) = page.contents {
            for object in contents {
                if let Some(key) = &object.key {
                    counter_total += 1;
                    let tags = client
                        .get_object_tagging()
                        .bucket(bucket)
                        .key(key)
                        .send()
                        .await?;

                    let no_gd_tag = tags
                        .tag_set()
                        .iter()
                        .find(|tag| tag.key() == "GuardDutyMalwareScanStatus")
                        .is_none();

                    if no_gd_tag {
                        if let Err(e) = copy_to_self(client, bucket, key).await {
                            eprintln!(
                                "Error: file '{}' could not be copied to bucket '{}'. Details: {}",
                                key, bucket, e
                            );
                        };
                        counter += 1;
                    }
                }
                if counter_total % 100 == 0 {
                    println!("processed {} objects, copied {}", counter_total, counter);
                }
            }
        }
    }

    Ok(counter)
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
