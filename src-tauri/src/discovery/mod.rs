use mdns_sd::{ServiceDaemon, ServiceInfo};
use std::collections::HashMap;
use std::net::IpAddr;

/// Guard that keeps the mDNS service registered for its lifetime.
/// When dropped, the service is automatically unregistered.
pub struct MdnsGuard {
    daemon: ServiceDaemon,
    fullname: String,
}

impl MdnsGuard {
    /// Register a LYNQO service on the LAN via mDNS/DNS-SD.
    ///
    /// - Service type: `_lynqo._tcp.local.`
    /// - Instance: `{device_name}._lynqo._tcp.local.`
    /// - Host: `{device_name}.local.`
    /// - Properties: version=1.0
    ///
    /// Pairing credentials are deliberately excluded: mDNS records are visible
    /// to every device on the local network.
    pub fn register(ip: &str, port: u16, device_name: &str) -> Result<Self, String> {
        let daemon =
            ServiceDaemon::new().map_err(|e| format!("Failed to create mDNS daemon: {}", e))?;

        // Sanitize device name for mDNS (replace spaces and special chars with hyphens)
        let safe_name: String = device_name
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' {
                    c
                } else {
                    '-'
                }
            })
            .collect();
        let safe_name = safe_name.trim_matches('-').to_string();
        let safe_name = if safe_name.is_empty() {
            "lynqo-device".to_string()
        } else {
            safe_name
        };

        let service_type = "_lynqo._tcp.local.";
        let instance_name = format!("{}.{}", safe_name, service_type);
        let host_name = format!("{}.local.", safe_name);

        let ip_addr: IpAddr = ip
            .parse()
            .map_err(|e| format!("Invalid IP address '{}': {}", ip, e))?;

        let mut properties = HashMap::new();
        properties.insert("version".to_string(), "1.0".to_string());

        let service_info = ServiceInfo::new(
            service_type,
            &safe_name,
            &host_name,
            ip_addr,
            port,
            properties,
        )
        .map_err(|e| format!("Failed to create service info: {}", e))?;

        daemon
            .register(service_info)
            .map_err(|e| format!("Failed to register mDNS service: {}", e))?;

        tracing::info!(
            "mDNS service registered: {} at {}:{}",
            instance_name,
            ip,
            port
        );

        Ok(Self {
            daemon,
            fullname: instance_name,
        })
    }
}

impl Drop for MdnsGuard {
    fn drop(&mut self) {
        if let Err(e) = self.daemon.unregister(&self.fullname) {
            tracing::error!(
                "Failed to unregister mDNS service '{}': {}",
                self.fullname,
                e
            );
        } else {
            tracing::info!("mDNS service unregistered: {}", self.fullname);
        }

        if let Err(e) = self.daemon.shutdown() {
            tracing::error!("Failed to shutdown mDNS daemon: {}", e);
        }
    }
}
