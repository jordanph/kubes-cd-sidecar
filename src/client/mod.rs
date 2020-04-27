use serde_derive::Serialize;
use reqwest::header::USER_AGENT;
use log::info;

#[derive(Serialize)]
pub struct CheckRunDetails {
  pub name: String,
  pub check_run_id: i32,
  pub repo_name: String,
  pub status: String,
  pub started_at: String,
  pub finished_at: Option<String>,
  pub logs: String,
  pub conclusion: String
}

pub struct KubesCDControllerClient {
    pub installation_id: u32,
    pub pod_name: String,
    pub base_url: String
}

impl KubesCDControllerClient {
    pub async fn update_check_run(&self, check_run_details: &CheckRunDetails) -> Result<(), Box<dyn std::error::Error>> {
        let request_url = format!("{}/update-check-run/{}", self.base_url, self.installation_id);

        info!("Updating check run...");

        reqwest::Client::new()
            .post(&request_url)
            .header(USER_AGENT, self.pod_name.clone())
            .json(&check_run_details)
            .send()
            .await?;

        Ok(())
    }
}
