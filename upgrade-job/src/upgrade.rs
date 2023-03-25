use crate::{
    common::{clients, error::Result},
    helm::upgrade::HelmUpgrade,
    opts::CliArgs,
};
use kube::{runtime::events::Recorder, Client as k8s_client};

pub(crate) mod data_plane;
pub(crate) mod utils;

pub(crate) async fn upgrade(opts: &CliArgs, event_recorder: &Recorder) -> Result<()> {
    let helm_upgrade = HelmUpgrade::default(opts).build()?;

    // Control plane containers are updated in this step.
    helm_upgrade.run(opts.umbrella_chart_dir(), opts.core_chart_dir())?;

    // Data plane containers are updated in this step.
    if opts.restart_data_plane() {
        data_plane::upgrade_data_plane(opts.namespace()).await?;
    }

    Ok(())
}
