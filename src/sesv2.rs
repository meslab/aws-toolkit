use crate::AppResult;
use aws_sdk_sesv2::{types::SuppressedDestinationSummary, Client};
use chrono::{DateTime, Utc};
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

    let mut emails = Vec::with_capacity(100);
    let now = chrono::Utc::now();

    while let Some(addresses) = sesv2_addresses_stream.next().await {
        debug!("Addresses: {:?}", addresses);

        emails.extend(
            addresses?
                .suppressed_destination_summaries()
                .iter()
                .filter_map(|address| email_address_filter(last_count_days, &now, address)),
        );
        thread::sleep(time::Duration::from_millis(1000));
    }
    Ok(emails)
}

fn email_address_filter(
    last_count_days: Option<u32>,
    now: &DateTime<Utc>,
    address: &SuppressedDestinationSummary,
) -> Option<(String, String, String)> {
    debug!("Address: {:?}", address);
    let timestamp = address.last_update_time();
    match last_count_days {
        Some(last) => get_email_if_match_time_interval(now, address, timestamp, last),
        None => get_email_suppression_record(address, timestamp),
    }
}

fn get_email_if_match_time_interval(
    now: &DateTime<Utc>,
    address: &SuppressedDestinationSummary,
    timestamp: &aws_sdk_ec2::primitives::DateTime,
    last: u32,
) -> Option<(String, String, String)> {
    let time_date = match DateTime::from_timestamp(timestamp.secs(), timestamp.subsec_nanos()) {
        Some(time_date) => time_date,
        None => {
            return None;
        }
    };

    let duration = *now - time_date;
    info!("Duration: {:?}", duration.num_days());

    if duration.num_days() < last as i64 {
        get_email_suppression_record(address, timestamp)
    } else {
        None
    }
}

fn get_email_suppression_record(
    address: &SuppressedDestinationSummary,
    timestamp: &aws_sdk_ec2::primitives::DateTime,
) -> Option<(String, String, String)> {
    Some((
        address.email_address().to_string(),
        address.reason().to_string(),
        timestamp.to_string(),
    ))
}
