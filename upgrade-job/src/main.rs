use crate::{
    common::{
        clients::{get_or_init_kube_client, get_or_init_rest_client},
        constants::DEFAULT_TRACING_FILTER,
        error::Error::{CliArgsParse, TracingSubscriberFilter},
    },
    opts::validators::{validate_helm_chart_dirs, validate_helm_release, validate_helmv3_in_path},
};
use clap::Parser;
use common::error::{must, Error, Result};
use k8s::event_helper::generate_event_recorder_for_k8s_job;
use kube::Client;
use openapi::tower::client::ApiClient;
use opts::CliArgs;
use snafu::{prelude::*, ErrorCompat, ResultExt};
use std::time::Duration;
use tokio::sync::OnceCell;
use tracing::error;
use tracing_error::ErrorLayer;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};
use upgrade::upgrade;
use url::Url;
use utils::ETCD_LABEL;

mod common;
mod helm;
mod k8s;
mod opts;
mod upgrade;

#[tokio::main]
async fn main() {
    must(init_logging());

    // Global resources -- storage REST client and kube client are initialized in this step.
    let opts = must(parse_cli_args().await);

    let event_recorder = must(generate_event_recorder_for_k8s_job(&opts).await);

    must(upgrade(&opts, &event_recorder).await);
}

/// Initialize logging components -- tracing.
fn init_logging() -> Result<()> {
    let fmt_layer = fmt::layer().with_target(false);
    let filter_layer = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(DEFAULT_TRACING_FILTER))
        .map_err(|e| TracingSubscriberFilter {
            source: e,
            filter: DEFAULT_TRACING_FILTER.to_string(),
        })?;

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .with(ErrorLayer::default())
        .init();

    Ok(())
}

/// This function handles the following tasks -- 1. Argument parsing, 2. Validating arguments whose
/// validation depends on other arguments.
pub(crate) async fn parse_cli_args() -> Result<CliArgs> {
    let opts = CliArgs::try_parse().map_err(|e| CliArgsParse { source: e })?;

    get_or_init_rest_client(opts.rest_endpoint().as_str()).await?;
    get_or_init_kube_client().await?;

    validate_helmv3_in_path()?;
    validate_helm_release(opts.release_name(), opts.namespace())?;
    validate_helm_chart_dirs(opts.umbrella_chart_dir(), opts.core_chart_dir())?;

    Ok(opts)
}
