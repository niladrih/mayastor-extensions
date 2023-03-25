use crate::common::error::Result;

use crate::common::error::Error::{
    EmptyStorageNodeSpec, GetStorageNode, ListStorageNodes, ListStorageVolumes,
};

use crate::common::clients::rest_client;

use k8s_openapi::api::core::v1::Pod;
use kube::{
    api::{DeleteParams, ListParams, ObjectList},
    Api, ResourceExt,
};
use openapi::models::CordonDrainState;
use snafu::{prelude::*, ResultExt};

/// Function to find whether any node drain is in progress.
pub(crate) async fn is_draining() -> Result<bool> {
    let mut is_draining = false;
    let nodes = rest_client()
        .nodes_api()
        .get_nodes()
        .await
        .map_err(|e| ListStorageVolumes { source: e })?;

    let nodelist = nodes.into_body();
    for node in nodelist {
        let node_spec = node
            .spec
            .ok_or_else(|| EmptyStorageNodeSpec { node_id: node.id })?;

        is_draining = match node_spec.cordondrainstate {
            Some(CordonDrainState::cordonedstate(_)) => false,
            Some(CordonDrainState::drainingstate(_)) => true,
            Some(CordonDrainState::drainedstate(_)) => false,
            None => false,
        };
        if is_draining {
            break;
        }
    }
    Ok(is_draining)
}

pub(crate) async fn is_node_cordoned(node_name: &str) -> Result<bool> {
    let node = rest_client()
        .nodes_api()
        .get_node(node_name)
        .await
        .map_err(|e| GetStorageNode {
            source: e,
            node_name: node_name.to_string(),
        })?;
    let node_body = node.into_body();
    let node_spec = &node_body.spec.ok_or_else(|| EmptyStorageNodeSpec {
        node_id: node_body.id,
    })?;
    let is_cordoned = match node_spec.cordondrainstate {
        Some(CordonDrainState::cordonedstate(_)) => true,
        Some(CordonDrainState::drainingstate(_)) => false,
        Some(CordonDrainState::drainedstate(_)) => false,
        None => false,
    };
    Ok(is_cordoned)
}

/// Function to check for any volume rebuild in progress across the cluster
pub(crate) async fn is_rebuilding() -> Result<bool> {
    // The number of volumes to get per request.
    let max_entries = 200;
    let mut starting_token = Some(0_isize);

    // The last paginated request will set the `starting_token` to `None`.
    while starting_token.is_some() {
        let vols = rest_client()
            .volumes_api()
            .get_volumes(max_entries, None, starting_token)
            .await
            .map_err(|e| ListStorageVolumes { source: e })?;

        let volumes = vols.into_body();
        starting_token = volumes.next_token;
        for volume in volumes.entries {
            if let Some(target) = &volume.state.target {
                if target
                    .children
                    .iter()
                    .any(|child| child.rebuild_progress.is_some())
                {
                    return Ok(true);
                }
            }
        }
    }
    Ok(false)
}

/// This function returns 'true' only if all of the containers in the Pods contained in the
/// ObjectList<Pod> have their Ready status.condition value set to true.
pub(crate) fn all_pods_are_ready(pod_list: ObjectList<Pod>) -> (bool, String, String) {
    let not_ready_warning = |pod_name: &String, namespace: &String| {
        tracing::warn!("Couldn't verify the ready condition of io-engine Pod '{}' in namespace '{}' to be true", pod_name, namespace);
    };
    for pod in pod_list.iter() {
        match &pod
            .status
            .as_ref()
            .and_then(|status| status.conditions.as_ref())
        {
            Some(conditions) => {
                for condition in *conditions {
                    if condition.type_.eq("Ready") && condition.status.eq("True") {
                        continue;
                    } else {
                        not_ready_warning(&pod.name_any(), &pod.namespace().unwrap_or_default());
                        return (false, pod.name_any(), pod.namespace().unwrap_or_default());
                    }
                }
            }
            None => {
                not_ready_warning(&pod.name_any(), &pod.namespace().unwrap_or_default());
                return (false, pod.name_any(), pod.namespace().unwrap_or_default());
            }
        }
    }
    (true, "".to_string(), "".to_string())
}
