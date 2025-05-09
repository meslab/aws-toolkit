use aws_config::Region;
use aws_sdk_autoscaling::Client as AutoScalingClient;
use aws_sdk_ec2::Client as Ec2Client;
use aws_sdk_ecs::Client as EcsClient;
use aws_sdk_elasticache::Client as ElasticacheClient;
use aws_sdk_elasticloadbalancingv2::Client as Elbv2Client;
use aws_sdk_rds::Client as RdsClient;

use aws_toolkit::{
    AppResult, autoscaling, client::initialize_client, ec2, ecs, elasticache, elbv2, rds,
};
use clap::Parser;
use log::{debug, info};
use std::io::Write;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[clap(
    version = "v0.2.6",
    author = "Anton Sidorov tonysidrock@gmail.com",
    about = "Scale down ECS cluster"
)]
struct Args {
    #[clap(short, long, required = true)]
    cluster: String,

    #[clap(short, long, default_value = "eu-central-1")]
    region: String,

    #[clap(short, long, default_value = "default")]
    profile: String,

    #[clap(short, long, default_value = "false")]
    delete: bool,

    #[clap(short = 'f', long, default_value = "false")]
    skip_final_rds_snapshot: bool,

    #[clap(short, long, default_value = "false")]
    scaledown: bool,
}

#[tokio::main]
async fn main() -> AppResult<()> {
    env_logger::init();

    let args = Args::parse();

    if args.delete && args.skip_final_rds_snapshot {
        // Show confirmation prompt
        print!(
            "Are you sure you want to DROP all databases without a final snapshot?\n(Type YES to confirm): "
        );
        std::io::stdout().flush()?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        if input.trim() != "YES" {
            println!("Aborted. Databases were not dropped.");
            return Ok(());
        }
        println!("Confirmation received. Proceeding with database drop...");
    }

    let region = Region::new(args.region.clone());

    let as_client = initialize_client::<AutoScalingClient>(region.clone(), &args.profile).await;
    let asgs = autoscaling::list_asgs(&as_client, &args.cluster, 0).await?;
    info!("ASGs: {:?}", asgs);

    let elc_client = initialize_client::<ElasticacheClient>(region.clone(), &args.profile).await;
    let replication_groups =
        elasticache::list_replication_groups(&elc_client, &args.cluster).await?;
    info!("Replication Groups: {:?}", replication_groups);

    let ecs_client = initialize_client::<EcsClient>(region.clone(), &args.profile).await;
    let services = ecs::get_service_arns(&ecs_client, &args.cluster, 0).await?;
    info!("Services: {:?}", services);

    let rds_client = initialize_client::<RdsClient>(region.clone(), &args.profile).await;
    let db_instances = rds::list_db_instances(&rds_client, &args.cluster).await?;
    info!("DB Instances: {:?}", db_instances);

    let elbv2_client = initialize_client::<Elbv2Client>(region.clone(), &args.profile).await;
    let load_balancers = elbv2::list_load_balancers(&elbv2_client, &args.cluster).await?;
    info!("Load Balancers: {:?}", load_balancers);

    let ec2_client = initialize_client::<Ec2Client>(region.clone(), &args.profile).await;
    let nat_gateways = ec2::get_nat_gateway_ids(&ec2_client, &args.cluster).await?;
    let ec2_instances = ec2::get_ec2_instances_ids(&ec2_client, &args.cluster).await?;

    if args.scaledown || args.delete {
        if !asgs.is_empty() {
            println!("Scaling down ASGs.");
            for asg in &asgs {
                autoscaling::scale_down_asg(&as_client, asg, 0).await?;
            }
        }
        if !services.is_empty() {
            println!("Scaling down ECS services.");
            for service in &services {
                ecs::scale_down_service(&ecs_client, &args.cluster, service, 0).await?;
            }
        }
        if !ec2_instances.is_empty() {
            println!("Terminating EC2 instances.");
            for ec2_instance in &ec2_instances {
                ec2::terminate_ec2_instance(&ec2_client, ec2_instance).await?;
            }
        }
        if !nat_gateways.is_empty() {
            println!("Deleting NAT gateways.");
            for nat_gateway in &nat_gateways {
                ec2::delete_nat_gateway(&ec2_client, nat_gateway).await?;
            }
        }
    }

    if args.scaledown && !db_instances.is_empty() {
        println!("Stopping RDS instances.");
        for db_instance in &db_instances {
            rds::stop_db_instance(&rds_client, db_instance).await?;
        }
    }

    if args.delete {
        if !replication_groups.is_empty() {
            println!("Deleting elasticache replication groups.");
            for replication_group in replication_groups {
                elasticache::delete_replication_group(&elc_client, &replication_group).await?;
            }
        }
        if !services.is_empty() {
            println!("Deleting ECS services.");
            for service in &services {
                ecs::delete_service(&ecs_client, &args.cluster, service).await?;
            }
        }
        if !db_instances.is_empty() {
            println!("Deleting RDS.");
            for db_instance in &db_instances {
                rds::disable_deletion_protection(&rds_client, db_instance).await?;
                if args.skip_final_rds_snapshot {
                    rds::delete_db_instance_skip_final_snapshot(&rds_client, db_instance).await?;
                } else {
                    rds::delete_db_instance_with_final_snapshot(&rds_client, db_instance).await?;
                }
            }
        }
        if !load_balancers.is_empty() {
            println!("Deleting load balancers.");
            for load_balancer in &load_balancers {
                elbv2::delete_load_balancer(&elbv2_client, load_balancer).await?;
            }
        }
    }

    debug!("Cluster: {} Region: {}.", &args.cluster, &args.region);

    Ok(())
}
