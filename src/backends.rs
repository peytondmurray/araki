use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;

use reqwest::{Client, header};

#[derive(Serialize, Deserialize)]
struct GitHubCreateRepositoryRequestBody {
    name: String,
    private: bool,
}

pub trait Backend {
    async fn is_existing_lockspec(&self, org: String, name: String)
    -> Result<bool, Box<dyn Error>>;
    async fn create_repository(&self, org: String, name: String) -> Result<(), Box<dyn Error>>;
}

pub struct GitHubBackend<'a> {
    api_url: &'a str,
    client: Client,
}

impl Backend for GitHubBackend<'_> {
    async fn is_existing_lockspec(
        &self,
        org: String,
        name: String,
    ) -> Result<bool, Box<dyn Error>> {
        let resp = self
            .client
            .get(format!("{}/repos/{}/{}", self.api_url, org, name))
            .send()
            .await?
            .json::<HashMap<String, String>>()
            .await?;

        println!("{resp:?}");
        Ok(resp.get("name").is_some())
    }
    async fn create_repository(&self, org: String, name: String) -> Result<(), Box<dyn Error>> {
        let body = GitHubCreateRepositoryRequestBody {
            name: name.clone(),
            private: true,
        };
        let status = self
            .client
            .post(format!("/orgs/{}/repos", org))
            .body(serde_json::to_string(&body)?)
            .send()
            .await?
            .status();

        if status.is_success() {
            Ok(())
        } else {
            Err(format!("Failed to create repository for {name}").into())
        }
    }
}

impl GitHubBackend<'_> {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            "Accept",
            header::HeaderValue::from_static("application/vnd.github+json"),
        );
        headers.insert(
            "Authorization",
            header::HeaderValue::from_static("Bearer <YOUR-TOKEN>"),
        );
        headers.insert(
            "X-GitHub-Api-Version",
            header::HeaderValue::from_static("2022-11-28"),
        );

        Ok(Self {
            api_url: "https://api.github.com/",
            client: Client::builder().default_headers(headers).build()?,
        })
    }
}
