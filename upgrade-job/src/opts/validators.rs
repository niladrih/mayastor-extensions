use crate::{
    common::{
        clients::kube_client,
        constants::{CORE_CHART_NAME, UMBRELLA_CHART_NAME},
        error::{
            Error::{
                FindingHelmChart, HelmCommand, HelmRelease, HelmVersion, NotADirectory, NotAFile,
                OpeningFile, RegexCompile, ValidateDirPath, ValidateFilePath, YamlParseFromFile,
                YamlStructure,
            },
            Result,
        },
    },
    helm::upgrade::HelmChartVariant,
};
use futures::StreamExt;
use k8s_openapi::api::{batch::v1::Job, core::v1::Namespace};
use kube::{
    api::{Api, ListParams, PostParams, ResourceExt},
    Client,
};
use regex::bytes::Regex;
use serde_yaml::Value;
use snafu::{prelude::*, ResultExt};
use std::{
    fs,
    ops::Deref,
    path::{Path, PathBuf},
    process::Command,
};

pub(crate) fn validate_helm_release(name: String, namespace: String) -> Result<()> {
    let command: &str = "helm";
    let args: Vec<String> = vec![
        "list".to_string(),
        "-n".to_string(),
        namespace.clone(),
        "--deployed".to_string(),
        "--short".to_string(),
    ];
    let output = Command::new(command)
        .args(args.clone())
        .output()
        .map_err(|e| HelmCommand {
            source: e,
            command: command.to_string(),
            args,
        })?;

    let regex = format!(r"(\n)?{}(\n)?", name.clone());
    if !Regex::new(regex.as_str())
        .map_err(|e| RegexCompile {
            source: e,
            expression: regex,
        })?
        .is_match(output.stdout.as_slice())
    {
        return Err(HelmRelease { name, namespace });
    }

    Ok(())
}

pub(crate) fn validate_helmv3_in_path() -> Result<()> {
    let command: &str = "helm";
    let args: Vec<String> = vec!["version".to_string(), "--short".to_string()];
    let output = Command::new(command)
        .args(args.clone())
        .output()
        .map_err(|e| HelmCommand {
            source: e,
            command: command.to_string(),
            args,
        })?;

    let output = output.stdout;
    let regex: &str = r"^(v3\.[0-9]+\.[0-9])";
    if !Regex::new(regex)
        .map_err(|e| RegexCompile {
            source: e,
            expression: regex.to_string(),
        })?
        .is_match(output.as_slice())
    {
        return Err(HelmVersion { version: output });
    }

    Ok(())
}

pub(crate) fn validate_helm_chart_dirs(
    umbrella_dir: Option<PathBuf>,
    core_dir: Option<PathBuf>,
) -> Result<()> {
    if let Some(path) = umbrella_dir {
        validate_helm_chart_variant_in_dir(HelmChartVariant::Umbrella, path)?;
    }
    if let Some(path) = core_dir {
        validate_helm_chart_variant_in_dir(HelmChartVariant::Core, path)?;
    }

    Ok(())
}

fn validate_helm_chart_variant_in_dir(
    chart_variant: HelmChartVariant,
    dir_path: PathBuf,
) -> Result<()> {
    let path_exists_and_is_dir = |path: PathBuf| -> Result<bool> {
        Ok(fs::metadata(path.clone())
            .map(|m| m.is_dir())
            .map_err(|e| ValidateDirPath { source: e, path })?)
    };

    let path_exists_and_is_file = |path: PathBuf| -> Result<bool> {
        Ok(fs::metadata(path.clone())
            .map(|m| m.is_file())
            .map_err(|e| ValidateFilePath { source: e, path })?)
    };

    let is_valid_helm_chart_variant =
        |chart_variant: HelmChartVariant, chart_name: String| -> bool {
            match chart_variant {
                HelmChartVariant::Umbrella => chart_name.eq(UMBRELLA_CHART_NAME),
                HelmChartVariant::Core => chart_name.eq(CORE_CHART_NAME),
            }
        };

    if !path_exists_and_is_dir(dir_path.clone())? {
        return Err(NotADirectory {
            path: dir_path.clone(),
        });
    }

    // Validate Chart.yaml file.
    let mut chart_yaml_path = dir_path.clone();
    chart_yaml_path.push("Chart.yaml");
    if !path_exists_and_is_file(chart_yaml_path.clone())? {
        return Err(NotAFile {
            path: chart_yaml_path.clone(),
        });
    }
    let chart_yaml_file =
        fs::File::open(chart_yaml_path.clone().deref()).map_err(|e| OpeningFile {
            source: e,
            filepath: chart_yaml_path.clone(),
        })?;
    let chart_yaml: Value =
        serde_yaml::from_reader(chart_yaml_file).map_err(|e| YamlParseFromFile {
            source: e,
            filepath: chart_yaml_path.clone(),
        })?;
    let chart_name_yaml_path = "name";
    if !is_valid_helm_chart_variant(
        chart_variant,
        chart_yaml[chart_name_yaml_path]
            .as_str()
            .ok_or_else(|| YamlStructure {
                yaml_path: chart_name_yaml_path.to_string(),
            })?
            .to_string(),
    ) {
        return Err(FindingHelmChart {
            path: dir_path.clone(),
        });
    }

    // Validate charts directory, it should exist if `helm dependency update` has been executed.
    let mut charts_dir_path = dir_path.clone();
    charts_dir_path.push("charts");
    if !path_exists_and_is_dir(charts_dir_path.clone())? {
        return Err(NotADirectory {
            path: charts_dir_path.clone(),
        });
    }

    // Validate values.yaml file.
    let mut values_yaml_path = dir_path.clone();
    values_yaml_path.push("values.yaml");
    if !path_exists_and_is_file(values_yaml_path.clone())? {
        return Err(NotAFile {
            path: values_yaml_path.clone(),
        });
    }

    // Validate README.md file.
    let mut readme_md_path = dir_path.clone();
    readme_md_path.push("README.md");
    if !path_exists_and_is_file(readme_md_path.clone())? {
        return Err(NotAFile {
            path: readme_md_path.clone(),
        });
    }

    // Validate crds directory.
    let mut crds_dir_path = dir_path.clone();
    crds_dir_path.push("crds");
    if !path_exists_and_is_dir(crds_dir_path.clone())? {
        return Err(NotADirectory {
            path: crds_dir_path.clone(),
        });
    }

    // Validate templates directory.
    let mut templates_dir_path = dir_path.clone();
    templates_dir_path.push("templates");
    if !path_exists_and_is_dir(templates_dir_path.clone())? {
        return Err(NotADirectory {
            path: templates_dir_path.clone(),
        });
    }

    Ok(())
}
