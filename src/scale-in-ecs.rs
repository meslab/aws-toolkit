use aws_config::Region;
use clap::Parser;
use log::{debug, info};
use aws_toolkit::{autoscaling, ecs, elasticache, elbv2, rds};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[clap(
    version = "v0.2.5",
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

    #[clap(short, long, default_value = "false")]
    scaledown: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let args = Args::parse();
    let region = Region::new(args.region.clone());

    let as_client = autoscaling::initialize_client(&args.region, &args.profile).await;
    let asgs = autoscaling::list_asgs(&as_client, &args.cluster, 0).await?;
    info!("ASGs: {:?}", asgs);

    let elc_client = elasticache::initialize_client(&args.region, &args.profile).await;
    let replication_groups =
        elasticache::list_replication_groups(&elc_client, &args.cluster).await?;
    info!("Replication Groups: {:?}", replication_groups);

    let ecs_client = ecs::initialize_client(region, &args.profile).await;
    let services = ecs::get_service_arns(&ecs_client, &args.cluster, 0).await?;
    info!("Services: {:?}", services);

    let rds_client = rds::initialize_client(&args.region, &args.profile).await;
    let db_instances = rds::list_db_instances(&rds_client, &args.cluster).await?;
    info!("DB Instances: {:?}", db_instances);

    let elbv2_client = elbv2::initialize_client(&args.region, &args.profile).await;
    let load_balancers = elbv2::list_load_balancers(&elbv2_client, &args.cluster).await?;
    info!("Load Balancers: {:?}", load_balancers);

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
        if !db_instances.is_empty() {
            println!("Stopping RDS instances.");
            for db_instance in &db_instances {
                rds::stop_db_instance(&rds_client, db_instance).await?;
            }
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
                rds::delete_db_instance(&rds_client, db_instance).await?;
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
