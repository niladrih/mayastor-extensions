use crate::common::error::{
    Error::{K8sClientGeneration, RestClientConfiguration, RestUrlParse},
    Result,
};
use kube::Client;
use openapi::tower::client::{ApiClient, Configuration as rest_config};
use snafu::{ResultExt, Snafu};
use std::time::Duration;
use tokio::sync::OnceCell;
use url::Url;

/// Thread-safe global storage REST API client container.
static REST_CLIENT: OnceCell<ApiClient> = OnceCell::const_new(); // Use rest_client().

pub(crate) async fn get_or_init_rest_client<'a>(rest_endpoint: &str) -> Result<&'a ApiClient> {
    let rest_endpoint = Url::try_from(rest_endpoint).map_err(|e| RestUrlParse {
        source: e,
        rest_endpoint: rest_endpoint.to_string(),
    })?;

    Ok(REST_CLIENT
        .get_or_try_init(|| async {
            let config = rest_config::builder()
                .with_timeout(Duration::from_secs(30))
                .with_tracing(true)
                .build_url(rest_endpoint.clone())
                .map_err(|_| RestClientConfiguration { rest_endpoint })?;

            Ok(ApiClient::new(config))
        })
        .await?)
}

pub(crate) fn rest_client() -> &'static ApiClient {
    REST_CLIENT.get().unwrap()
}

/// Thread-safe global Kubernetes REST API client container.
static KUBE_CLIENT: OnceCell<Client> = OnceCell::const_new(); // Use kube_client().

pub(crate) async fn get_or_init_kube_client() -> Result<Client> {
    Ok(KUBE_CLIENT
        .get_or_try_init(|| async {
            Ok(Client::try_default()
                .await
                .map_err(|e| K8sClientGeneration { source: e })?)
        })
        .await?
        .clone())
}

pub(crate) fn kube_client() -> Client {
    KUBE_CLIENT.get().unwrap().clone()
}
