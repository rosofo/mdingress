#[derive(clap::Parser)]
pub struct Config {
    #[arg(short, long)]
    ip_address: Option<String>,
    #[arg(long, default_value = "KUBERNETES_SERVICE_HOST")]
    ip_address_env: String,
}

impl Config {
    pub fn ip_address(&self) -> String {
        if let Some(addr) = self.ip_address.as_ref() {
            addr.clone()
        } else {
            if let Ok(addr) = std::env::var(&self.ip_address_env) {
                addr
            } else {
                panic!(
                    "IP address not provided, and not found in env var {}",
                    &self.ip_address_env
                );
            }
        }
    }
}
