use crate::constants::DeviceType;
use reqwest::{Client, ClientBuilder, Proxy};
use std::env;
#[allow(unused_imports)]
use std::time::Duration;

#[cfg(target_arch = "wasm32")]
pub fn http_client(device_type: DeviceType) -> Client {
    let proxy_url =
        env::var("SOCKS5_PROXY").unwrap_or_else(|_| "socks5://your-proxy-ip:port".to_string());

    let proxy = Proxy::all(&proxy_url).expect("Invalid SOCKS5 proxy URL");

    ClientBuilder::new()
        .user_agent(format!(
            "curl/8.7.1; {}; {}",
            device_type,
            env!("CARGO_PKG_VERSION")
        ))
        .proxy(proxy) // ✅ Using SOCKS5 proxy
        .build()
        .unwrap_or_default()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn http_client(device_type: DeviceType) -> Client {
    let proxy_url =
        env::var("SOCKS5_PROXY").unwrap_or_else(|_| "socks5://your-proxy-ip:port".to_string());

    let proxy = Proxy::all(&proxy_url).expect("Invalid SOCKS5 proxy URL");

    ClientBuilder::new()
        .timeout(Duration::from_secs(3))
        .cookie_store(true)
        .user_agent(format!(
            "curl/8.7.1; {}; {}",
            device_type,
            env!("CARGO_PKG_VERSION")
        ))
        .proxy(proxy) // ✅ Using SOCKS5 proxy
        .no_hickory_dns()
        .use_rustls_tls()
        .build()
        .unwrap_or_default()
}
