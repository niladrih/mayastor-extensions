/*
use std::iter::Map;
use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
pub(crate) struct ChartDependency {
    pub(crate) name: String,
    pub(crate) version: String,
    pub(crate) repository: Option<String>,
    pub(crate) condition: Option<String>,
    pub(crate) tags: Option<Vec<String>>,
    #[serde(rename(deserialize = "import-values"))]
    pub(crate) import_values: Option<Vec<String>>,
    pub(crate) alias: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct ChartMaintainer {
    pub(crate) name: String,
    pub(crate) email: Option<String>,
    pub(crate) url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ChartData {
    #[serde(rename(deserialize = "apiVersion"))]
    pub(crate) api_version: String,
    pub(crate) name: String,
    pub(crate) version: String,
    #[serde(rename(deserialize = "kubeVersion"))]
    pub(crate) kube_version: Option<String>,
    pub(crate) description: Option<String>,
    #[serde(rename(deserialize = "type"))]
    pub(crate) chart_type: Option<String>,
    pub(crate) keywords: Option<Vec<String>>,
    pub(crate) home: Option<String>,
    pub(crate) sources: Option<Vec<String>>,
    pub(crate) dependencies: Option<Vec<ChartDependency>>,
    pub(crate) icon: Option<String>,
    #[serde(rename(deserialize = "appVersion"))]
    pub(crate) app_version: Option<String>,
    pub(crate) deprecated: Option<String>,
    pub(crate) annotations: Map<String, String>,
}
*/