use std::net::{IpAddr, Ipv4Addr};

#[derive(Debug, Clone)]
pub(super) struct InterfaceCandidate {
    pub name: String,
    pub description: String,
    pub ip: Ipv4Addr,
    pub has_default_gateway: bool,
    pub is_default_route: bool,
}

pub(super) fn active_ipv4_interfaces() -> Vec<InterfaceCandidate> {
    let default_ip = local_ip_address::local_ip().ok().and_then(|ip| match ip {
        IpAddr::V4(ip) => Some(ip),
        IpAddr::V6(_) => None,
    });

    #[cfg(target_os = "windows")]
    {
        let mut interfaces = windows_active_ipv4_interfaces().unwrap_or_default();
        for interface in &mut interfaces {
            interface.is_default_route = default_ip == Some(interface.ip);
        }
        interfaces
    }

    #[cfg(not(target_os = "windows"))]
    {
        fallback_ipv4_interfaces(default_ip)
    }
}

fn is_usable_ipv4(ip: Ipv4Addr) -> bool {
    !ip.is_loopback() && !ip.is_link_local() && !ip.is_unspecified()
}

#[cfg(not(target_os = "windows"))]
fn fallback_ipv4_interfaces(default_ip: Option<Ipv4Addr>) -> Vec<InterfaceCandidate> {
    local_ip_address::list_afinet_netifas()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|(name, ip)| match ip {
            IpAddr::V4(ip) if is_usable_ipv4(ip) => Some(InterfaceCandidate {
                description: name.clone(),
                has_default_gateway: default_ip == Some(ip),
                is_default_route: default_ip == Some(ip),
                name,
                ip,
            }),
            _ => None,
        })
        .collect()
}

#[cfg(target_os = "windows")]
fn windows_active_ipv4_interfaces() -> Option<Vec<InterfaceCandidate>> {
    use std::{ptr, slice};
    use windows_sys::Win32::{
        Foundation::{ERROR_BUFFER_OVERFLOW, ERROR_SUCCESS},
        NetworkManagement::{
            IpHelper::{
                GetAdaptersAddresses, GAA_FLAG_INCLUDE_GATEWAYS, GAA_FLAG_SKIP_ANYCAST,
                GAA_FLAG_SKIP_DNS_SERVER, GAA_FLAG_SKIP_MULTICAST, IP_ADAPTER_ADDRESSES_LH,
            },
            Ndis::IfOperStatusUp,
        },
        Networking::WinSock::{IpDadStatePreferred, AF_INET, SOCKADDR_IN},
    };

    unsafe fn wide_string(value: *const u16) -> String {
        if value.is_null() {
            return String::new();
        }
        let mut length = 0usize;
        while unsafe { *value.add(length) } != 0 {
            length += 1;
        }
        String::from_utf16_lossy(unsafe { slice::from_raw_parts(value, length) })
    }

    let flags = GAA_FLAG_INCLUDE_GATEWAYS
        | GAA_FLAG_SKIP_ANYCAST
        | GAA_FLAG_SKIP_MULTICAST
        | GAA_FLAG_SKIP_DNS_SERVER;
    let mut size = 15_000u32;

    loop {
        // GetAdaptersAddresses requires the returned structure to be aligned.
        // Back the variable-sized region with its real structure type so this
        // remains correctly aligned on x86, x64, and ARM64 Windows targets.
        let element_size = std::mem::size_of::<IP_ADAPTER_ADDRESSES_LH>();
        let element_count = (size as usize).div_ceil(element_size).max(1);
        let mut buffer =
            vec![std::mem::MaybeUninit::<IP_ADAPTER_ADDRESSES_LH>::uninit(); element_count];
        let first_adapter = buffer.as_mut_ptr().cast::<IP_ADAPTER_ADDRESSES_LH>();
        let result = unsafe {
            GetAdaptersAddresses(AF_INET as u32, flags, ptr::null(), first_adapter, &mut size)
        };

        if result == ERROR_BUFFER_OVERFLOW {
            continue;
        }
        if result != ERROR_SUCCESS {
            tracing::warn!("GetAdaptersAddresses failed with Windows error {}", result);
            return None;
        }

        let mut interfaces = Vec::new();
        let mut adapter_ptr = first_adapter;
        while !adapter_ptr.is_null() {
            let adapter = unsafe { &*adapter_ptr };
            if adapter.OperStatus == IfOperStatusUp {
                let name = unsafe { wide_string(adapter.FriendlyName) };
                let description = unsafe { wide_string(adapter.Description) };
                let has_default_gateway = !adapter.FirstGatewayAddress.is_null();
                let mut unicast_ptr = adapter.FirstUnicastAddress;

                while !unicast_ptr.is_null() {
                    let unicast = unsafe { &*unicast_ptr };
                    let socket = unicast.Address.lpSockaddr;
                    if unicast.DadState == IpDadStatePreferred
                        && unicast.PreferredLifetime != 0
                        && !socket.is_null()
                        && unicast.Address.iSockaddrLength as usize
                            >= std::mem::size_of::<SOCKADDR_IN>()
                        && unsafe { (*socket).sa_family } == AF_INET
                    {
                        let socket_v4 = unsafe { &*(socket.cast::<SOCKADDR_IN>()) };
                        let raw = unsafe { socket_v4.sin_addr.S_un.S_addr };
                        let ip = Ipv4Addr::from(raw.to_ne_bytes());
                        if is_usable_ipv4(ip) {
                            interfaces.push(InterfaceCandidate {
                                name: name.clone(),
                                description: description.clone(),
                                ip,
                                has_default_gateway,
                                is_default_route: false,
                            });
                        }
                    }
                    unicast_ptr = unicast.Next;
                }
            }
            adapter_ptr = adapter.Next;
        }

        return Some(interfaces);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usable_ipv4_filter_rejects_non_routable_host_addresses() {
        assert!(is_usable_ipv4("192.168.10.4".parse().unwrap()));
        assert!(!is_usable_ipv4("127.0.0.1".parse().unwrap()));
        assert!(!is_usable_ipv4("169.254.3.4".parse().unwrap()));
        assert!(!is_usable_ipv4("0.0.0.0".parse().unwrap()));
    }
}
