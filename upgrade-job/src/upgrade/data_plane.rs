use crate::{
    common::{
        clients::{kube_client, rest_client},
        constants::{AGENT_CORE_LABEL, DRAIN_FOR_UPGRADE, IO_ENGINE_LABEL},
        error::{
            Error::{
                DrainStorageNode, EmptyPodNodeName, EmptyPodSpec, ListPodsWithLabel,
                PodDeleteError, StorageNodeUncordon, ValidatingPodRunningStatus,
            },
            Result,
        },
    },
    upgrade::utils::{all_pods_are_ready, is_draining, is_node_cordoned, is_rebuilding},
};
use k8s_openapi::api::core::v1::Pod;
use kube::{
    api::{DeleteParams, ListParams, ObjectList},
    Api, ResourceExt,
};
use snafu::{prelude::*, ResultExt};
use std::{ops::Deref, time::Duration};
use utils::{tracing_telemetry::trace::FutureExt, API_REST_LABEL, ETCD_LABEL};

/// Upgrade data plane by controlled restart of io-engine pods
pub(crate) async fn upgrade_data_plane(namespace: String) -> Result<()> {
    let pods: Api<Pod> = Api::namespaced(kube_client(), namespace.clone().as_str());

    let io_engine_listparam = ListParams::default().labels(IO_ENGINE_LABEL);
    let namespace = namespace.clone();
    let initial_io_engine_pod_list: ObjectList<Pod> = pods
        .list(&io_engine_listparam)
        .await
        .map_err(|e| ListPodsWithLabel {
            source: e,
            label: IO_ENGINE_LABEL.to_string(),
            namespace: namespace.clone(),
        })?;
    for pod in initial_io_engine_pod_list.iter() {
        // Fetch the node name on which the io-engine pod is running
        let node_name = pod
            .spec
            .as_ref()
            .ok_or_else(|| EmptyPodSpec {
                name: pod.name_any(),
                namespace: namespace.clone(),
            })?
            .node_name
            .as_ref()
            .ok_or_else(|| EmptyPodNodeName {
                name: pod.name_any(),
                namespace: namespace.clone(),
            })?
            .as_str();

        tracing::info!(
            pod.name = %pod.name_any(),
            node.name = %node_name,
            "Upgrade starting for data-plane pod"
        );

        let is_node_cordoned = is_node_cordoned(node_name).await?;

        // Issue node drain command
        issue_node_drain(node_name).await?;

        // Wait for node drain to complete across the cluster.
        wait_node_drain().await?;

        // Wait for any rebuild to complete.
        wait_for_rebuild().await?;

        // restart the data plane pod
        restart_data_plane(node_name, pod, namespace.clone()).await?;

        // Uncordon the drained node
        if !is_node_cordoned {
            uncordon_node(node_name).await?;
        }

        // validate the new pod is up and running
        verify_data_plane_pod_is_running(node_name, namespace.clone()).await?;

        // Validate the control plane pod is up and running
        is_control_plane_running(namespace.clone()).await?;
    }
    Ok(())
}

async fn uncordon_node(node_name: &str) -> Result<()> {
    rest_client()
        .nodes_api()
        .delete_node_cordon(node_name, DRAIN_FOR_UPGRADE)
        .await
        .map_err(|e| StorageNodeUncordon {
            source: e,
            node_name: node_name.to_string(),
        })?;

    tracing::info!(node.name = node_name, "Storage Node is uncordoned");

    Ok(())
}

/// Issue delete command on dataplane pods.
async fn restart_data_plane(node_name: &str, pod: &Pod, namespace: String) -> Result<()> {
    let pods: Api<Pod> = Api::namespaced(kube_client(), namespace.as_str());
    // Deleting the io-engine pod
    let pod_name = pod.name_any();
    tracing::info!(
        pod.name = pod_name.clone(),
        node.name = node_name,
        "Deleting the pod"
    );
    pods.delete(pod_name.as_str(), &DeleteParams::default())
        .await
        .map_err(|e| PodDeleteError {
            source: e,
            name: pod_name,
            node: node_name.to_string(),
        })?;
    Ok(())
}

/// Wait for the data plane pod to come up on the given node.
async fn wait_node_drain() -> Result<()> {
    while is_draining().await? {
        tokio::time::sleep(Duration::from_secs(10_u64)).await;
    }
    Ok(())
}

/// Wait for all the node drain process to complete.
async fn verify_data_plane_pod_is_running(node_name: &str, namespace: String) -> Result<()> {
    // Validate the new pod is up and running
    while is_data_plane_pod_running(node_name, namespace.clone()).await? {
        tokio::time::sleep(Duration::from_secs(10_u64)).await;
    }
    Ok(())
}

///  Wait for the rebuild to complete if any
async fn wait_for_rebuild() -> Result<()> {
    // Wait for 60 seconds for any rebuilds to kick in.
    tokio::time::sleep(Duration::from_secs(60_u64)).await;
    while is_rebuilding().await? {
        tokio::time::sleep(Duration::from_secs(10_u64)).await;
    }
    Ok(())
}

/// Issue the node drain command on the node.
async fn issue_node_drain(node_name: &str) -> Result<()> {
    rest_client()
        .nodes_api()
        .put_node_drain(node_name, DRAIN_FOR_UPGRADE)
        .await
        .map_err(|e| DrainStorageNode {
            source: e,
            node_name: node_name.to_string(),
        })?;

    tracing::info!(node.name = %node_name, "Drain started");

    Ok(())
}

async fn is_data_plane_pod_running(node: &str, namespace: String) -> Result<bool> {
    let mut data_plane_pod_running = false;
    let pods: Api<Pod> = Api::namespaced(kube_client(), namespace.clone().as_str());
    let io_engine_listparam = ListParams::default().labels(IO_ENGINE_LABEL);
    let initial_io_engine_pod_list: ObjectList<Pod> = pods
        .list(&io_engine_listparam)
        .await
        .map_err(|e| ListPodsWithLabel {
            source: e,
            label: IO_ENGINE_LABEL.to_string(),
            namespace: namespace.clone(),
        })?;
    //let data_plane_pod_running =
    for pod in initial_io_engine_pod_list.iter() {
        // Fetch the node name on which the io-engine pod is running
        let node_name = pod
            .spec
            .as_ref()
            .ok_or_else(|| EmptyPodSpec {
                name: pod.name_any(),
                namespace: namespace.clone(),
            })?
            .node_name
            .as_ref()
            .ok_or_else(|| EmptyPodNodeName {
                name: pod.name_any(),
                namespace: namespace.clone(),
            })?
            .as_str();
        if node != node_name {
            continue;
        } else {
            match pod
                .status
                .as_ref()
                .and_then(|status| status.conditions.as_ref())
            {
                Some(conditions) => {
                    for condition in conditions {
                        if condition.type_.eq("Ready") && condition.status.eq("True") {
                            data_plane_pod_running = true
                        } else {
                            data_plane_pod_running = false;
                        }
                    }
                }
                None => {
                    data_plane_pod_running = false;
                }
            }
        }
    }
    Ok(data_plane_pod_running)
}

async fn is_control_plane_running(namespace: String) -> Result<()> {
    let pods: Api<Pod> = Api::namespaced(kube_client(), namespace.clone().as_str());

    let pod_list: ObjectList<Pod> = pods
        .list(&ListParams::default().labels(AGENT_CORE_LABEL))
        .await
        .map_err(|e| ListPodsWithLabel {
            source: e,
            label: AGENT_CORE_LABEL.to_string(),
            namespace: namespace.clone(),
        })?;
    let core_result = all_pods_are_ready(pod_list);
    if !core_result.0 {
        return Err(ValidatingPodRunningStatus {
            name: core_result.1,
            namespace: core_result.2,
        });
    }

    let pod_list: ObjectList<Pod> = pods
        .list(&ListParams::default().labels(API_REST_LABEL))
        .await
        .map_err(|e| ListPodsWithLabel {
            source: e,
            label: API_REST_LABEL.to_string(),
            namespace: namespace.clone(),
        })?;
    let rest_result = all_pods_are_ready(pod_list);
    if !rest_result.0 {
        return Err(ValidatingPodRunningStatus {
            name: rest_result.1,
            namespace: rest_result.2,
        });
    }
    let pod_list: ObjectList<Pod> = pods
        .list(&ListParams::default().labels(ETCD_LABEL))
        .await
        .map_err(|e| ListPodsWithLabel {
            source: e,
            label: ETCD_LABEL.to_string(),
            namespace: namespace.clone(),
        })?;
    let etcd_result = all_pods_are_ready(pod_list);
    if !etcd_result.0 {
        return Err(ValidatingPodRunningStatus {
            name: etcd_result.1,
            namespace: etcd_result.2,
        });
    }

    Ok(())
}
