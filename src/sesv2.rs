use crate::AppResult;
use aws_sdk_sesv2::Client;
use chrono::DateTime;
use log::{debug, info};
use std::{thread, time};

pub async fn get_suppression_list(
    sesv2_client: &Client,
    last_count_days: Option<u32>,
) -> AppResult<Vec<(String, String, String)>> {
    let mut sesv2_addresses_stream = sesv2_client
        .list_suppressed_destinations()
        .page_size(1000)
        .into_paginator()
        .send();

    let mut emails = Vec::new();

    while let Some(addresses) = sesv2_addresses_stream.next().await {
        debug!("Addresses: {:?}", addresses);

        for address in addresses.unwrap().suppressed_destination_summaries() {
            debug!("Address: {:?}", address);
            let now = chrono::Utc::now();
            let timestamp = address.last_update_time();
            match last_count_days {
                None => {
                    emails.push((
                        address.email_address().to_string(),
                        address.reason().to_string(),
                        timestamp.to_string(),
                    ));
                }
                Some(last) => {
                    let time_date = match DateTime::from_timestamp(
                        timestamp.secs(),
                        timestamp.subsec_nanos(),
                    ) {
                        Some(time_date) => time_date,
                        None => {
                            return Err("Error parsing date".into());
                        }
                    };
                    let duration = now - time_date;
                    info!("Duration: {:?}", duration.num_days());
                    if duration.num_days() < last as i64 {
                        emails.push((
                            address.email_address().to_string(),
                            address.reason().to_string(),
                            timestamp.to_string(),
                        ));
                    }
                }
            }
        }
        thread::sleep(time::Duration::from_millis(1000));
    }
    // Err("Address not found".into())
    Ok(emails)
}
