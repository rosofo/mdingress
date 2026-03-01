mod mdns;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use eyre::{OptionExt, eyre};
use futures::{Stream, StreamExt, TryStreamExt};
use k8s_openapi::api::networking::v1::Ingress;
use kube::{
    Api, Client, Resource,
    runtime::{WatchStreamExt, events::Event, reflector::Lookup, watcher},
};
use tokio::time::sleep;
use tracing::{debug, info, level_filters::LevelFilter};
use tracing_subscriber::{EnvFilter, filter::Directive};

struct IngressMapper(HashMap<String, mdns::Service>, String);

impl IngressMapper {
    fn new(ip: String) -> Self {
        Self(HashMap::new(), ip)
    }

    fn register(&mut self, ingress: Ingress) -> eyre::Result<()> {
        let host = {
            let spec = ingress
                .spec
                .ok_or_else(|| eyre!("Ingress {:?} has no spec", ingress.metadata.name))?;
            let rules = spec
                .rules
                .ok_or_else(|| eyre!("Ingress {:?} has no rules", ingress.metadata.name))?;
            debug!("got rules: {:?}", rules);
            let Some(host) = rules
                .iter()
                .filter_map(|rule| rule.host.clone())
                .find(|host| host.ends_with(".local"))
            else {
                return Err(eyre!(
                    "Ingress {:?} has no host containing .local",
                    ingress.metadata.name
                ));
            };
            host
        };
        info!("Mapping {} to {}", &host, &self.1);
        let service = mdns::Service::new(&host, &self.1);
        self.0.insert(host, service);
        Ok(())
    }
}

async fn watch_ingresses(
    stream: impl Stream<Item = watcher::Result<watcher::Event<Ingress>>>,
) -> eyre::Result<()> {
    let mapper = Arc::new(Mutex::new(IngressMapper::new("192.168.1.111".to_string())));
    stream
        .applied_objects()
        .default_backoff()
        .try_for_each(|ing| {
            let mapper = mapper.clone();
            async move {
                info!("saw {:?}", ing.name());
                if let Err(err) = mapper.clone().lock().unwrap().register(ing) {
                    info!("Not mapping ingress: {}", err);
                };
                Ok(())
            }
        })
        .await?;
    Ok(())
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::DEBUG.into())
                .from_env()?,
        )
        .init();
    let client = Client::try_default().await?;
    debug!(
        "k8s server version: {:?}",
        client.apiserver_version().await?
    );

    let api: Api<Ingress> = Api::all(client);
    let config = watcher::Config::default();
    let ingress_watcher = watcher(api, config);
    let _task = tokio::spawn(watch_ingresses(ingress_watcher));

    loop {
        sleep(Duration::from_secs(1)).await
    }
}
