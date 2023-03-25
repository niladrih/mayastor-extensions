use crate::common::constants::PRODUCT;
use openapi::clients::tower::configuration as rest_config;
use snafu::{prelude::*, Backtrace, ErrorCompat, Snafu};
use std::path::{Path, PathBuf};
use url::Url;

/// For use with multiple fallible operations which may fail for different reasons, but are
/// defined withing the same scope and must return to the outer scope (calling scope) using
/// the try operator -- '?'.
#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
#[snafu(context(suffix(false)))]
pub(crate) enum Error {
    /// Error for when the storage REST API URL is parsed.
    #[snafu(display("Failed to {} REST API URL {}: {}", PRODUCT, rest_endpoint, source))]
    RestUrlParse {
        source: url::ParseError,
        rest_endpoint: String,
    },

    /// Error for when cli args are parsed.
    #[snafu(display("Failed to parse cli args: {}", source))]
    CliArgsParse { source: clap::error::Error },

    /// Error for when Kubernetes API client generation fails.
    #[snafu(display("Failed to generate kubernetes client: {}", source))]
    K8sClientGeneration { source: kube::Error },

    /// Error for when REST API configuration fails.
    #[snafu(display(
        "Failed to configure {} REST API client with endpoint {}",
        PRODUCT,
        rest_endpoint
    ))]
    RestClientConfiguration { rest_endpoint: Url },

    /// Error for use when parsing invalid tracing-subscriber filter directive.
    #[snafu(display(
        "Failed to create tracing-subscriber filter with directive {}: {}",
        filter,
        source
    ))]
    TracingSubscriberFilter {
        source: tracing_subscriber::filter::ParseError,
        filter: String,
    },

    /// Error for when a Helm command fails.
    #[snafu(display(
        "Failed to run Helm command, command: {}, args: {:?}, command_error: {}",
        command,
        args,
        source
    ))]
    HelmCommand {
        source: std::io::Error,
        command: String,
        args: Vec<String>,
    },

    /// Error for when regular expression parsing or compilation fails.
    #[snafu(display("Failed to compile regex {}: {}", expression, source))]
    RegexCompile {
        source: regex::Error,
        expression: String,
    },

    /// Error for when Helm v3.x.y is not present in $PATH.
    #[snafu(display("Helm version {} does not start with 'v3.x.y'", std::str::from_utf8(version).unwrap()))]
    HelmVersion { version: Vec<u8> },

    /// Error for when input Helm release is not found in the input namespace.
    #[snafu(display(
        "'deployed' Helm release {} not found in Namespace {}",
        name,
        namespace
    ))]
    HelmRelease { name: String, namespace: String },

    #[snafu(display("No input for {} helm chart's directory path", chart_name))]
    NoInputHelmChartDir { chart_name: String },

    #[snafu(display(".metadata.ownerReferences empty for Pod {} in {} namespace, while trying to find Pod's Job owner", pod_name, pod_namespace))]
    JobPodOwnerNotFound {
        pod_name: String,
        pod_namespace: String,
    },

    #[snafu(display(
        "Pod {} in {} namespace has too many owners, while trying to find Pod's Job owner",
        pod_name,
        pod_namespace
    ))]
    JobPodHasTooManyOwners {
        pod_name: String,
        pod_namespace: String,
    },

    #[snafu(display("Pod {} in {} namespace has an owner which is not a Job, while trying to find Pod's Job owner", pod_name, pod_namespace))]
    JobPodOwnerIsNotJob {
        pod_name: String,
        pod_namespace: String,
    },

    #[snafu(display("Failed to parse YAML {}: {}", std::str::from_utf8(input_yaml).unwrap(), source))]
    YamlParseFromSlice {
        source: serde_yaml::Error,
        input_yaml: Vec<u8>,
    },

    #[snafu(display("Failed to parse YAML at {}: {}", filepath.display(), source))]
    YamlParseFromFile {
        source: serde_yaml::Error,
        filepath: PathBuf,
    },

    #[snafu(display(
        "Helm chart release {} in Namespace {} use an unsupported chart variant: {}",
        release_name,
        namespace,
        chart_name
    ))]
    DetermineChartVariant {
        release_name: String,
        namespace: String,
        chart_name: String,
    },

    #[snafu(display("Failed to validate directory path {}: {}", path.display(), source))]
    ValidateDirPath {
        source: std::io::Error,
        path: PathBuf,
    },

    #[snafu(display("Failed to validate filepath {}: {}", path.display(), source))]
    ValidateFilePath {
        source: std::io::Error,
        path: PathBuf,
    },

    #[snafu(display("{} is not a directory", path.display()))]
    NotADirectory { path: PathBuf },

    #[snafu(display("{} is not a file", path.display()))]
    NotAFile { path: PathBuf },

    #[snafu(display("Failed to open file {}: {}", filepath.display(), source))]
    OpeningFile {
        source: std::io::Error,
        filepath: PathBuf,
    },

    #[snafu(display("Failed to find valid Helm chart in path {}", path.display()))]
    FindingHelmChart { path: PathBuf },

    #[snafu(display("Failed to find chart version as semver in chart name {}", chart_name))]
    FindingSemverInChartName { chart_name: String },

    #[snafu(display(
        "Failed to GET Pod {} in namespace {}: {}",
        pod_name,
        pod_namespace,
        source
    ))]
    GetPod {
        source: kube::Error,
        pod_name: String,
        pod_namespace: String,
    },

    #[snafu(display(
        "Failed to list Pods with label {} in namespace {}: {}",
        label,
        namespace,
        source
    ))]
    ListPodsWithLabel {
        source: kube::Error,
        label: String,
        namespace: String,
    },

    #[snafu(display("Failed get .spec from Pod {} in Namespace {}", name, namespace))]
    EmptyPodSpec { name: String, namespace: String },

    #[snafu(display(
        "Failed get .spec.nodeName from Pod {} in Namespace {}",
        name,
        namespace
    ))]
    EmptyPodNodeName { name: String, namespace: String },

    #[snafu(display("Failed to uncordon {} Node {}: {}", PRODUCT, node_name, source))]
    StorageNodeUncordon {
        source: openapi::tower::client::Error<openapi::models::RestJsonError>,
        node_name: String,
    },

    #[snafu(display("Failed get delete Pod {} from Node {}: {}", name, node, source))]
    PodDeleteError {
        source: kube::Error,
        name: String,
        node: String,
    },

    #[snafu(display("Failed to list {} Nodes: {}", PRODUCT, source))]
    ListStorageNodes {
        source: openapi::tower::client::Error<openapi::models::RestJsonError>,
    },

    #[snafu(display("Failed to list {} Node {}: {}", PRODUCT, node_name, source))]
    GetStorageNode {
        source: openapi::tower::client::Error<openapi::models::RestJsonError>,
        node_name: String,
    },

    #[snafu(display("Failed to get {} Node {}", PRODUCT, node_id))]
    EmptyStorageNodeSpec { node_id: String },

    #[snafu(display("Failed to list {} Volumes: {}", PRODUCT, source))]
    ListStorageVolumes {
        source: openapi::tower::client::Error<openapi::models::RestJsonError>,
    },

    #[snafu(display("Failed to drain {} Node {}: {}", PRODUCT, node_name, source))]
    DrainStorageNode {
        source: openapi::tower::client::Error<openapi::models::RestJsonError>,
        node_name: String,
    },

    #[snafu(display("Pod {} in Namespace {} is not running", name, namespace))]
    ValidatingPodRunningStatus { name: String, namespace: String },

    #[snafu(display("Failed to parse YAML path {}", yaml_path))]
    YamlStructure { yaml_path: String },
}

pub(crate) type Result<T, E = Error> = std::result::Result<T, E>;

pub(crate) fn must<T>(output: Result<T>) -> T {
    if let Err(error) = output {
        tracing::error!(?error, "Failed to upgrade");
        std::process::exit(-1);
    }
    output.unwrap()
}
