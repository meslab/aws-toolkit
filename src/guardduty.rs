use crate::AppResult;
use aws_sdk_guardduty::Client;
use aws_sdk_guardduty::types::DetectorFeatureResult;
use log::debug;

pub async fn get_buckets_with_guardduty_enabled(client: &Client) -> AppResult<bool> {
    let detectors = client.list_detectors().send().await?;

    let detector_id = match detectors.detector_ids().first() {
        Some(id) => id,
        None => {
            debug!("GuardDuty is NOT enabled (no detector found)");
            return Ok(false);
        }
    };

    let detector = client
        .get_detector()
        .detector_id(detector_id)
        .send()
        .await?;

    println!("GuardDuty status: {:?}", &detector.status());

    for feature in detector.features() {
        let name = feature.name();
        if matches!(name, Some(DetectorFeatureResult::S3DataEvents)) {
            println!("S3 protection status: {:?}", feature.status());
        } else {
            return Ok(true);
        }
    }

    Ok(false)
}

pub async fn list_malware_protected_buckets(
    client: &Client,
    include: &[String],
    exclude: &[String],
) -> AppResult<Vec<String>> {
    let mut next_token = None;
    let mut bucket_names = Vec::new();

    let include = include.to_vec();
    let exclude = exclude.to_vec();

    let filter = |name: &str| {
        include.iter().any(|x| name.contains(x)) && exclude.iter().all(|x| !name.contains(x))
    };

    loop {
        let resp = client
            .list_malware_protection_plans()
            .set_next_token(next_token.clone())
            .send()
            .await?;

        for plan_id in resp.malware_protection_plans() {
            let plan = client
                .get_malware_protection_plan()
                .malware_protection_plan_id(
                    plan_id.malware_protection_plan_id().unwrap_or_default(),
                )
                .send()
                .await?;

            if let Some(resource) = plan.protected_resource()
                && let Some(s3_bucket) = resource.s3_bucket()
                && let Some(name) = s3_bucket.bucket_name()
                && filter(name)
            {
                bucket_names.push(name.to_owned());
            }
        }

        next_token = resp.next_token().map(ToString::to_string);
        if next_token.is_none() {
            break;
        }
    }

    bucket_names.sort();
    bucket_names.dedup();
    Ok(bucket_names)
}
