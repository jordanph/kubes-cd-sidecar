use k8s_openapi::api::core::v1::Pod;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::Time;
use kube::{
    api::{Api, LogParams},
    Client,
};
use log::{error, info};
use std::time;

mod client;

use client::{CheckRunDetails, KubesCDControllerClient};

#[tokio::main]
async fn main() {
    let _ = pretty_env_logger::try_init();

    let pod_name = std::env::var("POD_NAME").unwrap();

    let installation_id = std::env::var("INSTALLATION_ID")
        .unwrap()
        .parse::<u32>()
        .unwrap();

    let kubes_cd_controller_base_url = std::env::var("KUBES_CD_CONTROLLER_BASE_URL").unwrap();

    let kubes_cd_controller = KubesCDControllerClient {
        installation_id,
        pod_name: &pod_name,
        base_url: &kubes_cd_controller_base_url,
    };

    info!("Polling the pods...");

    match poll_pod(&pod_name, &kubes_cd_controller).await {
        Ok(_) => info!("All pods have finished! Spinning down..."),
        Err(err) => {
            error!("Error occurred while polling pods {}", err);

            std::process::exit(1);
        }
    };
}

async fn poll_pod<'a>(
    pod_name: &str,
    controller_client: &'a KubesCDControllerClient<'a>
) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::infer().await?;
    let namespace = std::env::var("NAMESPACE").unwrap_or_else(|_| "default".into());

    let pods: Api<Pod> = Api::namespaced(client, &namespace);

    info!("Waiting for pod to finish to return completion state...");

    let mut completed_container: Vec<String> = Vec::new();

    loop {
        let pod: Pod = pods.get(pod_name).await?;

        let pod_status = pod.status.unwrap();
        let pod_spec = pod.spec.unwrap();

        let container_statuses = pod_status.container_statuses.as_ref().unwrap();

        for container_status in container_statuses.iter() {
            if completed_container.contains(&container_status.name)
                || container_status.name == "kubes-cd-sidecar"
            {
                continue;
            }

            let container_state = container_status.state.as_ref().unwrap();

            if container_state.running.is_some() {
                info!("Pod is still running...");
            } else if let Some(terminated) = container_state.terminated.as_ref() {
                info!("Pod has finished!");

                let container_envs = pod_spec
                    .containers
                    .iter()
                    .find(|container| container.name == container_status.name)
                    .unwrap()
                    .env
                    .as_ref()
                    .unwrap();

                let check_run_id = container_envs
                    .iter()
                    .find(|env| env.name == "CHECK_RUN_ID")
                    .as_ref()
                    .unwrap()
                    .value
                    .as_ref();

                let Time(started_at) = terminated.started_at.as_ref().unwrap();
                let Time(finished_at) = terminated.finished_at.as_ref().unwrap();

                let conclusion = if terminated.exit_code == 0 {
                    "success"
                } else {
                    "failure"
                };

                let container_name = container_status.name.clone();

                let logs = get_container_logs(&pods, pod_name, &container_name).await?;

                let rfc_started_at = &started_at.to_rfc3339();
                let rfc_finished_at = &finished_at.to_rfc3339();

                let check_run_details = CheckRunDetails {
                    check_run_id: check_run_id.unwrap().parse().unwrap(),
                    status: "completed",
                    started_at: rfc_started_at,
                    finished_at: Some(rfc_finished_at),
                    logs: &logs,
                    conclusion,
                };

                controller_client
                    .update_check_run(&check_run_details)
                    .await?;

                completed_container.push(container_name)
            } else {
                info!("Pod is still waiting...");
            }
        }

        if completed_container.len() > container_statuses.len() - 1 {
            info!("All containers have finished!");

            break;
        }

        tokio::time::delay_for(time::Duration::from_secs(5)).await;
    }

    Ok(())
}

async fn get_container_logs(
    pod: &Api<Pod>,
    pod_name: &str,
    container_name: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut lp = LogParams::default();
    lp.follow = true;
    lp.timestamps = true;
    lp.container = Some(container_name.to_string());

    Ok(pod.logs(pod_name, &lp).await?)
}
