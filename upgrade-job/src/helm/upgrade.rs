use crate::{
    common::{
        constants::{CORE_CHART_NAME, UMBRELLA_CHART_NAME},
        error::{
            Error::{
                DetermineChartVariant, NoInputHelmChartDir, OpeningFile, RegexCompile,
                YamlParseFromFile, YamlStructure,
            },
            Result,
        },
    },
    helm::client::HelmClient,
    opts::CliArgs,
};
use clap::{builder::TypedValueParser, ValueEnum};
use regex::Regex;
use serde_yaml::Value;
use snafu::{prelude::*, ResultExt};
use std::{
    borrow::Borrow,
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone)]
pub(crate) enum HelmChartVariant {
    Umbrella,
    Core,
}

pub(crate) struct HelmUpgrade {
    chart_variant: HelmChartVariant,
    release_name: String,
    client: HelmClient,
}

impl HelmUpgrade {
    pub(crate) fn default(opts: &CliArgs) -> Self {
        Self {
            chart_variant: HelmChartVariant::Umbrella,
            release_name: opts.release_name(),
            client: HelmClient::default(opts),
        }
    }

    pub(crate) fn build(mut self) -> Result<Self> {
        let chart = self.client.release_info(self.release_name.clone())?.chart();

        let umbrella_chart_regex = format!(r"^({}-[0-9]+\.[0-9]+\.[0-9]+)$", UMBRELLA_CHART_NAME);
        let core_chart_regex = format!(r"^({}-[0-9]+\.[0-9]+\.[0-9]+)$", CORE_CHART_NAME);

        if Regex::new(umbrella_chart_regex.as_str())
            .map_err(|e| RegexCompile {
                source: e,
                expression: umbrella_chart_regex,
            })?
            .is_match(chart.as_str())
        {
            self.chart_variant = HelmChartVariant::Umbrella
        } else if Regex::new(core_chart_regex.as_str())
            .map_err(|e| RegexCompile {
                source: e,
                expression: core_chart_regex,
            })?
            .is_match(chart.as_str())
        {
            self.chart_variant = HelmChartVariant::Core
        } else {
            return Err(NoInputHelmChartDir {
                chart_name: chart.to_string(),
            });
        }

        Ok(self)
    }

    pub(crate) fn run(
        &self,
        umbrella_chart_dir: Option<PathBuf>,
        core_chart_dir: Option<PathBuf>,
    ) -> Result<()> {
        // Get image tag from the target Helm chart.
        let chart_dir: PathBuf;
        match self.chart_variant {
            HelmChartVariant::Umbrella => {
                chart_dir = umbrella_chart_dir
                    .clone()
                    .ok_or_else(|| NoInputHelmChartDir {
                        chart_name: UMBRELLA_CHART_NAME.to_string(),
                    })?;
            }
            HelmChartVariant::Core => {
                chart_dir = core_chart_dir.clone().ok_or_else(|| NoInputHelmChartDir {
                    chart_name: CORE_CHART_NAME.to_string(),
                })?;
            }
        }
        let mut values_yaml_path = chart_dir.clone();
        values_yaml_path.push("values.yaml");
        let values_yaml_file =
            fs::File::open(values_yaml_path.clone()).map_err(|e| OpeningFile {
                source: e,
                filepath: values_yaml_path.clone(),
            })?;
        let values_yaml: Value =
            serde_yaml::from_reader(values_yaml_file).map_err(|e| YamlParseFromFile {
                source: e,
                filepath: values_yaml_path.clone(),
            })?;

        let image_tag: &str;

        let image_key: &str = "image";
        let tag_key: &str = "tag";
        match self.chart_variant {
            HelmChartVariant::Umbrella => {
                let parent_key_umbrella = CORE_CHART_NAME;
                image_tag = values_yaml[parent_key_umbrella][image_key][tag_key]
                    .as_str()
                    .ok_or_else(|| YamlStructure {
                        yaml_path: format!(".{}.{}.{}", parent_key_umbrella, image_key, tag_key),
                    })?;
            }
            HelmChartVariant::Core => {
                image_tag =
                    values_yaml[image_key][tag_key]
                        .as_str()
                        .ok_or_else(|| YamlStructure {
                            yaml_path: format!(".{}.{}", image_key, tag_key),
                        })?;
            }
        }

        // Helm upgrade flags, reuse all values, except for the image tag.
        let mut upgrade_args: Vec<String> = Vec::with_capacity(2);
        let mut image_tag_arg: String = "--set ".to_string();
        match self.chart_variant {
            HelmChartVariant::Umbrella => {
                // This turns out to be `--set CORE_CHART_NAME.image.tag=`.
                image_tag_arg.push_str(CORE_CHART_NAME);
                image_tag_arg.push_str(".image.tag=");
            }
            HelmChartVariant::Core => {
                // This turns out to be `--set image.tag=`.
                image_tag_arg.push_str("image.tag=");
            }
        }
        image_tag_arg.push_str(image_tag);

        upgrade_args.push(image_tag_arg);
        upgrade_args.push("--reuse-values".to_string());
        upgrade_args.push("--wait".to_string());

        let chart_dir: String;
        match self.chart_variant {
            HelmChartVariant::Umbrella => {
                chart_dir = umbrella_chart_dir
                    .expect("invalid character")
                    .to_string_lossy()
                    .to_string()
            }
            HelmChartVariant::Core => {
                chart_dir = core_chart_dir
                    .expect("invalid character")
                    .to_string_lossy()
                    .to_string()
            }
        }

        Ok(self
            .client
            .upgrade(self.release_name.clone(), chart_dir, Some(upgrade_args))?)
    }
}
