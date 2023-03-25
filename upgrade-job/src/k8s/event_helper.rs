use crate::{
    common::{
        clients::kube_client,
        constants::KUBE_EVENT_REPORTER_NAME,
        error::{
            Error::{GetPod, JobPodHasTooManyOwners, JobPodOwnerIsNotJob, JobPodOwnerNotFound},
            Result,
        },
    },
    opts::CliArgs,
};
use futures::StreamExt;
use k8s_openapi::{
    api::core::v1::{ObjectReference, Pod},
    apimachinery::pkg::apis::meta::v1::OwnerReference,
};
use kube::{
    api::{Api, ListParams, PostParams},
    runtime::{
        events::{Recorder, Reporter},
        reflector::ObjectRef,
    },
    Client,
};
use snafu::{prelude::*, ResultExt};
use std::ops::Deref;

pub(crate) async fn generate_event_recorder_for_k8s_job(opts: &CliArgs) -> Result<Recorder> {
    let pod_client: Api<Pod> = Api::namespaced(kube_client(), opts.namespace().as_str());
    let pod = pod_client
        .get(opts.pod_name().as_str())
        .await
        .map_err(|e| GetPod {
            source: e,
            pod_name: opts.pod_name(),
            pod_namespace: opts.namespace(),
        })?;

    if !pod.metadata.owner_references.is_some() {
        return Err(JobPodOwnerNotFound {
            pod_name: opts.pod_name(),
            pod_namespace: opts.namespace(),
        });
    }
    let pod_owner = pod.metadata.owner_references.clone().unwrap()[0].clone();
    if !(pod.metadata.owner_references.unwrap().len() == 1) {
        return Err(JobPodHasTooManyOwners {
            pod_name: opts.pod_name(),
            pod_namespace: opts.namespace(),
        });
    }
    if !pod_owner.kind.eq("Job") {
        return Err(JobPodOwnerIsNotJob {
            pod_name: opts.pod_name(),
            pod_namespace: opts.namespace(),
        });
    }

    let owner_job_obj_ref = ObjectReference {
        api_version: Some(pod_owner.api_version),
        kind: Some(pod_owner.kind),
        name: Some(pod_owner.name),
        namespace: Some(opts.namespace()),
        uid: Some(pod_owner.uid),
        field_path: None,
        resource_version: None,
    };
    Ok(Recorder::new(
        kube_client(),
        KUBE_EVENT_REPORTER_NAME.into(),
        owner_job_obj_ref,
    ))
}
