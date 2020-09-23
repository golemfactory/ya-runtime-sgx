use chrono::{DateTime, Utc};

use futures::prelude::*;
use std::time::Duration;
use yarapi::rest;

//const PACKAGE : &str = "hash:sha3:5e50bf012d5f7466d23d8557dc7b570134ac8e7c0ff66077f432a0d7:http://yacn.dev.golem.network.:8000/trusted-voting-mgr-8e7c0ff66077f432a0d7.ywasi";
const PACKAGE : &str = "hash:sha3:4365f695f264bfe4c4ec527874bc03fec4b318fa955d60f8dfc95c6c:http://yacn.dev.golem.network.:8000/trustless-voting-mgr-18fa955d60f8dfc95c6c.ywasi";

pub async fn list_nodes(
    market: rest::Market,
    subnet: &str,
    runtime: &str,
) -> anyhow::Result<Vec<serde_json::Value>> {
    let deadline = Utc::now() + chrono::Duration::minutes(15);
    let ts = deadline.timestamp_millis();
    let props = serde_json::json!({
        "golem.node.id.name": "operator",
        "golem.node.debug.subnet": subnet,
        "golem.srv.comp.task_package": PACKAGE,
        "golem.srv.comp.expiration": ts
    });
    let constraints = format!(
        "(&(golem.runtime.name={runtime})(golem.node.debug.subnet={subnet}))",
        runtime = runtime,
        subnet = subnet
    );
    let subscrption = market.subscribe(&props, &constraints).await?;
    let mut collected_nodes = Vec::new();
    let result = actix_rt::time::timeout(
        Duration::from_secs(15),
        subscrption.proposals().try_for_each(|p| {
            if p.is_response() {
                let name = p
                    .props()
                    .as_object()
                    .and_then(|m| m.get("golem.node.id.name"));
                let subnet = p
                    .props()
                    .as_object()
                    .and_then(|m| m.get("golem.node.debug.subnet"));
                collected_nodes.push(
                    serde_json::json!({"nodeId": p.issuer_id(), "name": name, "subnet": subnet}),
                );
            }
            let props = props.clone();
            let constraints = constraints.clone();
            async move {
                if p.is_response() {
                    let _ignore = p.reject_proposal().await;
                } else {
                    let _ignore = p.counter_proposal(&props, &constraints).await;
                };
                Ok(())
            }
        }),
    )
    .await;
    log::error!("r={:?}", result);
    Ok(collected_nodes)
}

pub async fn create_agreement(
    market: rest::Market,
    subnet: &str,
    runtime: &str,
    deadline: DateTime<Utc>,
) -> anyhow::Result<rest::Agreement> {
    let ts = deadline.timestamp_millis();
    let props = serde_json::json!({
        "golem.node.id.name": "operator",
        "golem.node.debug.subnet": subnet,
        "golem.srv.comp.task_package": PACKAGE,
        "golem.srv.comp.expiration": ts
    });
    let constraints = format!(
        "(&(golem.runtime.name={runtime})(golem.node.debug.subnet={subnet}))",
        runtime = runtime,
        subnet = subnet
    );
    let subscrption = market.subscribe(&props, &constraints).await?;

    log::info!("constraints={}", constraints);

    let proposals = subscrption.proposals();
    futures::pin_mut!(proposals);
    while let Some(proposal) = proposals.try_next().await? {
        log::info!(
            "got proposal: {} -- from: {}, draft: {:?}",
            proposal.id(),
            proposal.issuer_id(),
            proposal.state()
        );
        if proposal.is_response() {
            let agreement = proposal.create_agreement(deadline).await?;
            log::info!("created agreement {}", agreement.id());
            if let Err(e) = agreement.confirm().await {
                log::error!("wait_for_approval failed: {:?}", e);
                continue;
            }
            return Ok(agreement);
        }
        let id = proposal.counter_proposal(&props, &constraints).await?;
        log::info!("got: {}", id);
    }
    anyhow::bail!("agreement not found")
}
