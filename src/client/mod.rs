use log::info;
use reqwest::header::USER_AGENT;
use serde_derive::Serialize;

#[derive(Serialize)]
pub struct CheckRunDetails<'a> {
    pub check_run_id: i32,
    pub repo_name: &'a str,
    pub status: &'a str,
    pub finished_at: Option<&'a str>,
    pub logs: &'a str,
    pub conclusion: &'a str,
}

#[derive(Serialize)]
pub struct PodFinishedSuccessfullyRequest<'a> {
    pub step_section: i32,
    pub repo_name: &'a str,
    pub commit_sha: &'a str,
}

pub struct KubesCDControllerClient<'a> {
    pub installation_id: u32,
    pub pod_name: &'a str,
    pub base_url: &'a str,
}

impl<'a> KubesCDControllerClient<'a> {
    pub async fn update_check_run<'b>(
        &self,
        check_run_details: &CheckRunDetails<'b>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let request_url = format!(
            "{}/update-check-run/{}",
            self.base_url, self.installation_id
        );

        info!("Updating check run...");

        reqwest::Client::new()
            .post(&request_url)
            .header(USER_AGENT, self.pod_name)
            .json(&check_run_details)
            .send()
            .await?;

        Ok(())
    }

    pub async fn notify_finished_successfully<'b>(
        &self,
        pod_finished_successfully_request: &PodFinishedSuccessfullyRequest<'b>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let request_url = format!(
            "{}/pod-finished/{}",
            self.base_url, self.installation_id
        );

        info!("Notifying all steps completed successfully...");

        reqwest::Client::new()
            .post(&request_url)
            .header(USER_AGENT, self.pod_name)
            .json(&pod_finished_successfully_request)
            .send()
            .await?;

        Ok(())
    }
}
