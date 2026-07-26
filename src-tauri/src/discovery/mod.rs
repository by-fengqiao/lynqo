use mdns_sd::{ServiceDaemon, ServiceInfo};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::net::IpAddr;

/// Guard that keeps the mDNS service registered for its lifetime.
/// When dropped, the service is automatically unregistered.
pub struct MdnsGuard {
    daemon: ServiceDaemon,
    fullname: String,
}

/// Return the exact DNS-safe host label used by both mDNS registration and
/// connection information shown to the user.
pub fn safe_host_label(device_name: &str) -> String {
    let mut safe_name = String::new();
    let mut previous_was_separator = false;
    for character in device_name.chars() {
        if character.is_ascii_alphanumeric() {
            safe_name.push(character.to_ascii_lowercase());
            previous_was_separator = false;
        } else if !safe_name.is_empty() && !previous_was_separator {
            safe_name.push('-');
            previous_was_separator = true;
        }
    }
    let safe_name = safe_name.trim_matches('-');
    let base = if safe_name.is_empty() {
        "lynqo-device"
    } else {
        safe_name
    };
    let digest = Sha256::digest(device_name.as_bytes());
    let suffix = format!(
        "{:02x}{:02x}{:02x}{:02x}",
        digest[0], digest[1], digest[2], digest[3]
    );
    // DNS labels are limited to 63 octets. The slug is ASCII, so truncating
    // by characters is also byte-safe. A stable suffix prevents non-ASCII
    // names from all collapsing to the same `lynqo-device` label.
    let max_base_len = 63 - 1 - suffix.len();
    let base = base
        .chars()
        .take(max_base_len)
        .collect::<String>()
        .trim_matches('-')
        .to_string();
    format!("{base}-{suffix}")
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

        let safe_name = safe_host_label(device_name);

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

#[cfg(test)]
mod tests {
    use super::safe_host_label;

    #[test]
    fn host_label_is_stable_and_dns_safe() {
        let ascii = safe_host_label("Feng Qiao's PC");
        assert!(ascii.starts_with("feng-qiao-s-pc-"));
        assert_eq!(ascii, safe_host_label("Feng Qiao's PC"));
        assert!(safe_host_label("  ").starts_with("lynqo-device-"));
        assert_ne!(safe_host_label("风桥"), safe_host_label("林桥"));
        assert!(safe_host_label(&"a".repeat(200)).len() <= 63);
        assert!(safe_host_label("LYNQO-01").starts_with("lynqo-01-"));
    }
}
