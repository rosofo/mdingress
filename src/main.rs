mod config;
mod mdns;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use clap::Parser;
use eyre::{OptionExt, eyre};
use futures::{Stream, StreamExt, TryStreamExt};
use k8s_openapi::api::networking::v1::Ingress;
use kube::{
    Api, Client, Resource,
    runtime::{WatchStreamExt, events::Event, reflector::Lookup, watcher},
};
use tokio::time::sleep;
use tracing::{debug, error, info, level_filters::LevelFilter};
use tracing_subscriber::{EnvFilter, filter::Directive};

use crate::config::Config;

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
        self.0.insert(
            ingress
                .metadata
                .uid
                .expect("expected a uid for the ingress"),
            service,
        );
        Ok(())
    }

    fn unregister(&mut self, uid: impl AsRef<str>) -> eyre::Result<()> {
        if let Some(svc) = self.0.remove(uid.as_ref()) {
            info!("Unregistering service for ingress uid:{}", uid.as_ref());
            drop(svc);
            Ok(())
        } else {
            Err(eyre!(
                "No service to unregister for ingress uid:{}",
                uid.as_ref()
            ))
        }
    }
}

async fn watch_ingresses(
    stream: impl Stream<Item = watcher::Result<watcher::Event<Ingress>>>,
    ip_addr: String,
) -> eyre::Result<()> {
    let mapper = Arc::new(Mutex::new(IngressMapper::new(ip_addr)));

    stream
        .default_backoff()
        .try_for_each(|ing| {
            let mapper = mapper.clone();
            async move {
                match ing {
                    watcher::Event::Delete(ing) => {
                        info!("{:?} deleted, unregistering service...", ing.name());
                        if let Err(err) = mapper
                            .clone()
                            .lock()
                            .unwrap()
                            .unregister(ing.uid().expect("expected a uid for the ingress"))
                        {
                            error!("Failed to unregister ingress: {}", err);
                        };
                    }
                    watcher::Event::Apply(ing) | watcher::Event::InitApply(ing) => {
                        info!("{:?} added, checking for `.local` host", ing.name());
                        if let Err(err) = mapper.clone().lock().unwrap().register(ing) {
                            info!("Not mapping ingress: {}", err);
                        };
                    }
                    _ => {}
                }
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

    let config = Config::parse();
    let ip_addr = config.ip_address();

    let client = Client::try_default().await?;
    debug!(
        "k8s server version: {:?}",
        client.apiserver_version().await?
    );

    let api: Api<Ingress> = Api::all(client);
    let config = watcher::Config::default();
    let ingress_watcher = watcher(api, config);
    let _task = tokio::spawn(watch_ingresses(ingress_watcher, ip_addr));

    loop {
        sleep(Duration::from_secs(1)).await
    }
}
