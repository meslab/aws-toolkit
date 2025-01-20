use crate::AppResult;
use aws_sdk_ec2::Client;
use log::debug;

pub async fn get_nat_gateway_ids(client: &Client, cluster: &str) -> AppResult<Vec<String>> {
    let mut nat_gateway_ids: Vec<String> = Vec::new();
    let mut nat_gateway_stream = client
        .describe_nat_gateways()
        .max_results(100)
        .into_paginator()
        .send();

    while let Some(nat_gateways) = nat_gateway_stream.next().await {
        debug!("NAT Gateways: {:?}", nat_gateways);
        for nat_gateway in nat_gateways?.nat_gateways() {
            if nat_gateway.tags().iter().any(|t| {
                t.value()
                    .expect("Cannot extract tag value.")
                    .contains(&cluster)
            }) {
                nat_gateway_ids.push(
                    nat_gateway
                        .nat_gateway_id()
                        .expect("Cannot extract gateway id.")
                        .to_owned(),
                );
            }
        }
    }
    Ok(nat_gateway_ids)
}

#[async_recursion::async_recursion]
pub async fn delete_nat_gateway(client: &Client, gateway_id: &str) -> AppResult<()> {
    match client
        .delete_nat_gateway()
        .nat_gateway_id(gateway_id)
        .send()
        .await
    {
        Ok(_) => Ok(()),
        _ => {
            tokio::time::sleep(std::time::Duration::from_secs(15)).await;
            delete_nat_gateway(client, &gateway_id).await?;
            Ok(())
        }
    }
}
