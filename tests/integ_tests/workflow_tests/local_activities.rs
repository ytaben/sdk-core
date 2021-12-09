use futures::future::join_all;
use std::time::Duration;
use temporal_sdk_core::prototype_rust_sdk::{LocalActivityOptions, WfContext, WorkflowResult};
use temporal_sdk_core_protos::coresdk::AsJsonPayloadExt;
use test_utils::CoreWfStarter;

pub async fn echo(e: String) -> anyhow::Result<String> {
    Ok(e)
}

pub async fn one_local_activity_wf(mut ctx: WfContext) -> WorkflowResult<()> {
    let initial_workflow_time = ctx.workflow_time().expect("Workflow time should be set");
    ctx.local_activity(LocalActivityOptions {
        activity_type: "echo_activity".to_string(),
        input: "hi!".as_json_payload().expect("serializes fine"),
        ..Default::default()
    })
    .await;
    // Verify LA execution advances the clock
    assert!(initial_workflow_time < ctx.workflow_time().unwrap());
    Ok(().into())
}

#[tokio::test]
async fn one_local_activity() {
    let wf_name = "one_local_activity";
    let mut starter = CoreWfStarter::new(wf_name);
    let mut worker = starter.worker().await;
    worker.register_wf(wf_name.to_owned(), one_local_activity_wf);
    worker.register_activity("echo_activity", echo);

    worker
        .submit_wf(wf_name.to_owned(), wf_name.to_owned(), vec![])
        .await
        .unwrap();
    worker.run_until_done().await.unwrap();
    starter.shutdown().await;
}

pub async fn local_act_concurrent_with_timer_wf(mut ctx: WfContext) -> WorkflowResult<()> {
    let la = ctx.local_activity(LocalActivityOptions {
        activity_type: "echo_activity".to_string(),
        input: "hi!".as_json_payload().expect("serializes fine"),
        ..Default::default()
    });
    let timer = ctx.timer(Duration::from_secs(1));
    tokio::join!(la, timer);
    Ok(().into())
}

#[tokio::test]
async fn local_act_concurrent_with_timer() {
    let wf_name = "local_act_concurrent_with_timer";
    let mut starter = CoreWfStarter::new(wf_name);
    let mut worker = starter.worker().await;
    worker.register_wf(wf_name.to_owned(), local_act_concurrent_with_timer_wf);
    worker.register_activity("echo_activity", echo);

    worker
        .submit_wf(wf_name.to_owned(), wf_name.to_owned(), vec![])
        .await
        .unwrap();
    worker.run_until_done().await.unwrap();
    starter.shutdown().await;
}

pub async fn local_act_then_timer_then_wait(mut ctx: WfContext) -> WorkflowResult<()> {
    let la = ctx.local_activity(LocalActivityOptions {
        activity_type: "echo_activity".to_string(),
        input: "hi!".as_json_payload().expect("serializes fine"),
        ..Default::default()
    });
    ctx.timer(Duration::from_secs(1)).await;
    la.await;
    Ok(().into())
}

#[tokio::test]
async fn local_act_then_timer_then_wait_result() {
    let wf_name = "local_act_then_timer_then_wait_result";
    let mut starter = CoreWfStarter::new(wf_name);
    let mut worker = starter.worker().await;
    worker.register_wf(wf_name.to_owned(), local_act_then_timer_then_wait);
    worker.register_activity("echo_activity", echo);

    worker
        .submit_wf(wf_name.to_owned(), wf_name.to_owned(), vec![])
        .await
        .unwrap();
    worker.run_until_done().await.unwrap();
    starter.shutdown().await;
}

#[tokio::test]
async fn long_running_local_act_with_timer() {
    let wf_name = "long_running_local_act_with_timer";
    let mut starter = CoreWfStarter::new(wf_name);
    starter.wft_timeout(Duration::from_secs(1));
    let mut worker = starter.worker().await;
    worker.register_wf(wf_name.to_owned(), local_act_then_timer_then_wait);
    worker.register_activity("echo_activity", |str: String| async {
        tokio::time::sleep(Duration::from_secs(4)).await;
        Ok(str)
    });

    worker
        .submit_wf(wf_name.to_owned(), wf_name.to_owned(), vec![])
        .await
        .unwrap();
    worker.run_until_done().await.unwrap();
    starter.shutdown().await;
}

pub async fn local_act_fanout_wf(mut ctx: WfContext) -> WorkflowResult<()> {
    let las: Vec<_> = (1..=50)
        .map(|i| {
            ctx.local_activity(LocalActivityOptions {
                activity_type: "echo_activity".to_string(),
                input: format!("Hi {}", i)
                    .as_json_payload()
                    .expect("serializes fine"),
                ..Default::default()
            })
        })
        .collect();
    ctx.timer(Duration::from_secs(1)).await;
    join_all(las).await;
    Ok(().into())
}

#[tokio::test]
async fn local_act_fanout() {
    let wf_name = "local_act_fanout";
    let mut starter = CoreWfStarter::new(wf_name);
    starter.max_local_at(1);
    let mut worker = starter.worker().await;
    worker.register_wf(wf_name.to_owned(), local_act_fanout_wf);
    worker.register_activity("echo_activity", echo);

    worker
        .submit_wf(wf_name.to_owned(), wf_name.to_owned(), vec![])
        .await
        .unwrap();
    worker.run_until_done().await.unwrap();
    starter.shutdown().await;
}