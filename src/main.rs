use log::{info, error};
use k8s_openapi::api::core::v1::Pod;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::Time;
use std::time;
use kube::{
    api::{Api,LogParams},
    Client
};

use std::collections::HashMap;

mod client;

use client::{CheckRunDetails, KubesCDControllerClient};

#[tokio::main]
async fn main() {
    let _ = pretty_env_logger::try_init();

    let check_run_pod_name =  std::env::var("CHECK_RUN_POD_NAME_MAP")
        .map(|check_run_str| extract_map(check_run_str)).unwrap();

    let pod_name = std::env::var("POD_NAME").unwrap();

    let installation_id = std::env::var("INSTALLATION_ID").unwrap().parse::<u32>().unwrap();
    
    let repo_name = std::env::var("REPO_NAME").unwrap();

    let kubes_cd_controller_base_url = std::env::var("KUBES_CD_CONTROLLER_BASE_URL").unwrap();

    let kubes_cd_controller = KubesCDControllerClient {
        installation_id: installation_id,
        pod_name: pod_name.clone(),
        base_url: kubes_cd_controller_base_url
    };

    info!("Polling the pods...");

    match poll_pod(pod_name, check_run_pod_name, kubes_cd_controller, repo_name).await {
        Ok(_) => info!("All pods have finished! Spinning down..."),
        Err(err) => {
            error!("Error occurred while polling pods {}", err);

            std::process::exit(1);
        }
    };
}

fn extract_map(check_run_map: String) -> HashMap<String, i32> {
    let mut hashmap: HashMap<String, i32> = HashMap::new();

    for check_map_pair in check_run_map.split(',') {
        let split_map: Vec<&str> = check_map_pair.split('=').collect();

        hashmap.insert(split_map[0].to_string(), split_map[1].to_string().parse::<i32>().unwrap());
    }

    hashmap
}

async fn poll_pod(pod_name: String, container_check_id_map: HashMap<String, i32>, controller_client: KubesCDControllerClient, repo_name: String) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::infer().await?;
    let namespace = std::env::var("NAMESPACE").unwrap_or("default".into());

    let pods: Api<Pod> = Api::namespaced(client, &namespace);

    info!("Waiting for pod to finish to return completion state...");

    let mut completed_container: Vec<String> = Vec::new();

    loop {
        let maybe_pod = pods.get(&pod_name).await?;

        let pod_status = maybe_pod.status.unwrap();

        let container_statuses = pod_status.container_statuses.unwrap();

        for container in container_statuses.into_iter() {
            if completed_container.contains(&container.name) || container.name == "kubes-cd-sidecar" {
                continue;
            }

            let container_state = container.state.unwrap();

            if let Some(_) = container_state.running {
                info!("Pod is still running...");
            } else if let Some(terminated) = container_state.terminated {
                info!("Pod has finished!");

                let Time(started_at) = terminated.started_at.unwrap();
                let Time(finished_at) = terminated.finished_at.unwrap();

                let conclusion = if terminated.exit_code == 0 {
                    "success"
                } else {
                    "failure"
                };

                let name = container.name.clone();

                let logs = get_container_logs(&pods, &pod_name, &name).await?;

                let check_run_details = CheckRunDetails {
                    check_run_id: container_check_id_map[&name],
                    name: container.name,
                    repo_name: repo_name.clone(),
                    status: "completed".to_string(),
                    started_at: started_at.to_rfc3339(),
                    finished_at: Some(finished_at.to_rfc3339()),
                    logs: logs,
                    conclusion: conclusion.to_string()
                };

                controller_client.update_check_run(&check_run_details).await?;

                completed_container.push(name.clone())
            } else {
                info!("Pod is still waiting...");
            }
        }

        if completed_container.len() > container_check_id_map.len() - 1 {
            info!("All containers have finished!");

            break;
        }
        
        tokio::time::delay_for(time::Duration::from_secs(5)).await;
    }

    Ok(())
}

async fn get_container_logs(pod: &Api<Pod>, pod_name: &String, container_name: &String) -> Result<String, Box<dyn std::error::Error>> {
    let mut lp = LogParams::default();
    lp.follow = true;
    lp.timestamps = true;
    lp.container = Some(container_name.clone());

    Ok(pod.logs(pod_name, &lp).await?)
}