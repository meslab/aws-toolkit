use aws_config::default_provider::credentials::DefaultCredentialsChain;
use aws_config::Region;
use aws_sdk_autoscaling::{Client as AutoScalingClient, Config as AutoScalingConfig};
use aws_sdk_codecommit::{Client as CodeCommitClient, Config as CodeCommitConfig};
use aws_sdk_ecs::{Client as EcsClient, Config as EcsConfig};
use aws_sdk_elasticache::{Client as ElasticacheClient, Config as ElasticacheConfig};
use aws_sdk_elasticloadbalancingv2::{Client as Elbv2Client, Config as Elbv2Config};
use aws_sdk_rds::{Client as RdsClient, Config as RdsConfig};
use aws_sdk_sesv2::{Client as Sesv2Client, Config as Sesv2Config};

// Define a generic trait for AWS clients
pub trait AwsClientBuilder<TConfig, TClient> {
    fn from_config(config: TConfig) -> TClient;
    fn build_config(region: Region, credentials_provider: DefaultCredentialsChain) -> TConfig;
}

// Macro to implement AwsClientBuilder for multiple services
macro_rules! impl_aws_client_builder {
    ($client:ty, $config:ty) => {
        impl AwsClientBuilder<$config, $client> for $client {
            fn from_config(config: $config) -> $client {
                <$client>::from_conf(config)
            }

            fn build_config(
                region: Region,
                credentials_provider: DefaultCredentialsChain,
            ) -> $config {
                <$config>::builder()
                    .credentials_provider(credentials_provider)
                    .region(region)
                    .build()
            }
        }
    };
}

// Implement the trait for AWS SDK clients
impl_aws_client_builder!(AutoScalingClient, AutoScalingConfig);
impl_aws_client_builder!(EcsClient, EcsConfig);
impl_aws_client_builder!(RdsClient, RdsConfig);
impl_aws_client_builder!(Sesv2Client, Sesv2Config);
impl_aws_client_builder!(CodeCommitClient, CodeCommitConfig);
impl_aws_client_builder!(ElasticacheClient, ElasticacheConfig);
impl_aws_client_builder!(Elbv2Client, Elbv2Config);

// Generic initialization function
pub async fn initialize_client<TConfig, TClient, C: AwsClientBuilder<TConfig, TClient>>(
    region: Region,
    profile: &str,
) -> TClient {
    let credentials_provider = DefaultCredentialsChain::builder()
        .profile_name(profile)
        .build()
        .await;

    let config = C::build_config(region, credentials_provider);
    C::from_config(config)
}
