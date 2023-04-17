use std::error::Error;
use chrono::{ LocalResult, Local, Duration, TimeZone };
use reqwest;
use serde_json::Value;
use notify_rust::{ Notification };
use dotenv::dotenv;
use open;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();

    let vercel_api_bearer_token = std::env
        ::var("VERCEL_API_BEARER_TOKEN")
        .expect("VERCEL_API_BEARER_TOKEN must be set");

    let mut ready_notified = false;

    loop {
        println!("Checking for new deployment...");
        let res = get_last_deployment(&vercel_api_bearer_token).await?;
        println!("Response: {}", res["deployments"][0]["created"]);
        let deployment_time_ms = res["deployments"][0]["created"].as_i64().unwrap();
        let deployment_time = match Local.timestamp_millis_opt(deployment_time_ms) {
            LocalResult::Single(dt) => dt,
            LocalResult::None => Local::now(),
            LocalResult::Ambiguous(_, _) => Local::now(),
        };

        let deployment_time_str = deployment_time.format("%Y-%m-%d %H:%M:%S").to_string();
        println!("Last deployment time: {}", deployment_time_str);

        let state = res["deployments"][0]["state"].as_str().unwrap();

        let now = Local::now();
        let diff = now.signed_duration_since(deployment_time);

        if state != "READY" {
            if diff < Duration::minutes(3) {
                if state == "BUILDING" {
                    Notification::new()
                        .summary(res["deployments"][0]["name"].as_str().unwrap())
                        .body("Deployment is building...")
                        .icon("/images/rucelify.png")
                        .timeout(0)
                        .show()?;
                }
            }
            ready_notified = false;
            tokio::time::sleep(tokio::time::Duration::from_secs(15)).await;
        } else if !ready_notified {
            Notification::new()
                .summary(res["deployments"][0]["name"].as_str().unwrap())
                .body("Deployment is ready!")
                .icon("/images/rucelify.png")
                .timeout(0)
                .show()?;

            let project_name = res["deployments"][0]["name"].as_str().unwrap();
            let project_domains = get_project_domains(
                &vercel_api_bearer_token,
                &project_name
            ).await?;
            let project_domain = project_domains["domains"][0]["name"].as_str().unwrap();

            if let Err(e) = open::that(format!("https://{}", project_domain)) {
                eprintln!("Failed to open deployment URL: {}", e);
            }

            ready_notified = true;
            tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
        }
    }
}

async fn get_last_deployment(token: &str) -> Result<Value, Box<dyn Error>> {
    println!("{}", token);
    let client = reqwest::Client::new();
    let res = client
        .get("https://api.vercel.com/v6/now/deployments")
        .header("Authorization", format!("Bearer {}", token))
        .send().await?
        .text().await?;
    Ok(serde_json::from_str::<Value>(&res).unwrap())
}

async fn get_project_domains(token: &str, project_name: &str) -> Result<Value, Box<dyn Error>> {
    println!("{}", token);
    let client = reqwest::Client::new();

    let res = client
        .get(format!("https://api.vercel.com/v8/projects/{}/domains", project_name))
        .header("Authorization", format!("Bearer {}", token))
        .send().await?
        .text().await?;
    Ok(serde_json::from_str::<Value>(&res).unwrap())
}