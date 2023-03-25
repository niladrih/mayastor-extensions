use crate::{
    common::error::{
        Error::{FindingSemverInChartName, HelmCommand, RegexCompile, YamlParseFromSlice},
        Result,
    },
    CliArgs,
};
use k8s_openapi::serde;
use regex::Regex;
use serde::Deserialize;
use snafu::ResultExt;
use std::{
    path::{Path, PathBuf},
    process::Command,
};
use tracing::info;

#[derive(Clone, Deserialize)]
pub(crate) struct HelmReleaseElement {
    name: String,
    //    namespace: String,
    //    revision: String,
    //    updated: String,
    //    status: String,
    chart: String,
    //    app_version: String,
}

impl HelmReleaseElement {
    pub(crate) fn name(&self) -> String {
        self.name.clone()
    }
    pub(crate) fn chart(&self) -> String {
        self.chart.clone()
    }
    /*
       pub(crate) fn chart_version(&self) -> Result<String> {
           let regex = r"([0-9]+\.[0-9]+\.[0-9]+)$";
           Ok(
               Regex::new(regex)
                   .context(RegexCompileSnafu { expression: regex })?
                   .find(self.chart.as_str())
                   .ok_or_else(|| FindingSemverInChartName { chart_name: self.chart.clone() })?
                   .as_str()
           )
       }
    */
}

pub(crate) struct HelmClient {
    namespace: String,
}

impl HelmClient {
    pub(crate) fn default(opts: &CliArgs) -> Self {
        Self {
            namespace: opts.namespace(),
        }
    }

    pub(crate) fn list(
        &self,
        maybe_extra_args: Option<Vec<String>>,
    ) -> Result<Vec<HelmReleaseElement>> {
        let command: &str = "helm";
        let mut args: Vec<String> = vec![
            "list".to_string(),
            "-n".to_string(),
            self.namespace.clone(),
            "--deployed".to_string(),
        ];
        if let Some(extra_args) = maybe_extra_args {
            for arg in extra_args.into_iter() {
                args.push(arg);
            }
        }
        // Because this flag has to be at the end for it to work.
        args.push("-o".to_string());
        args.push("yaml".to_string());

        let output = Command::new(command)
            .args(args.clone())
            .output()
            .map_err(|e| HelmCommand {
                source: e,
                command: command.to_string(),
                args,
            })?;

        let output = output.stdout;

        Ok(
            serde_yaml::from_slice(output.as_slice()).map_err(|e| YamlParseFromSlice {
                source: e,
                input_yaml: output,
            })?,
        )
    }

    pub(crate) fn upgrade(
        &self,
        release_name: String,
        chart_dir: String,
        maybe_extra_args: Option<Vec<String>>,
    ) -> Result<()> {
        let command: &str = "helm";
        let mut args: Vec<String> = vec![
            "upgrade".to_string(),
            release_name,
            chart_dir,
            "-n".to_string(),
            self.namespace.clone(),
        ];

        if let Some(extra_args) = maybe_extra_args {
            for arg in extra_args.into_iter() {
                args.push(arg);
            }
        }

        let output = Command::new(command)
            .args(args.clone())
            .output()
            .map_err(|e| HelmCommand {
                source: e,
                command: command.to_string(),
                args,
            })?;

        info!("Helm upgrade successful!");

        Ok(())
    }

    pub(crate) fn release_info(&self, release_name: String) -> Result<HelmReleaseElement> {
        let release_list = self.list(None)?;

        for release in release_list.into_iter() {
            if release.name().eq(&release_name) {
                return Ok(release.clone());
            }
        }

        // The code reaching this line means that the release is not there, even though we might
        // have seen that it exists some while back when validating the input Helm release
        // name in the input Namespace.
        panic!(
            "It is expected that there exists a Helm release {} in Namespace {}, but it does not exist",
            release_name,
            self.namespace,
        );
    }
}
