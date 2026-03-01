use std::borrow::Cow;
use std::sync::Arc;

use tokio::process::Command;
use tokio::sync::Notify;
use tracing::debug;

pub struct Service {
    notify: Arc<Notify>,
}

impl Service {
    pub fn new(host: &str, ip: &str) -> Self {
        let notify = Arc::new(Notify::new());

        let notify2 = Arc::clone(&notify);
        let mut cmd = Command::new("avahi-publish-address");
        cmd.args([
            "-a",
            "-R", // Do not add a reverse record (IP -> hostname), so we can have multiple hosts pointing at same IP
            host, ip,
        ]);
        let task = tokio::spawn(async move {
            if let Err(err) = register_address(notify2, cmd).await {
                tracing::error!("Error mapping address: {}", err);
            }
        });

        Self { notify }
    }
}
async fn register_address(notify: Arc<Notify>, mut cmd: Command) -> eyre::Result<()> {
    let mut proc = cmd.spawn()?;

    notify.notified().await;
    debug!("killing avahi proc for service");

    proc.kill().await?;
    Ok(())
}

impl Drop for Service {
    fn drop(&mut self) {
        self.notify.notify_one();
    }
}
