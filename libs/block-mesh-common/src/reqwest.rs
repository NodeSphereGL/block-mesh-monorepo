use crate::constants::DeviceType;
use reqwest::{Client, ClientBuilder, Proxy};
use std::env;
#[allow(unused_imports)]
use std::time::Duration;
use tracing::info;

#[cfg(target_arch = "wasm32")]
pub fn http_client(device_type: DeviceType) -> Client {
    let proxy_url =
        env::var("SOCKS5_PROXY").unwrap_or_else(|_| "socks5://your-proxy-ip:port".to_string());

    info!("Request wasm32 client with proxy URL: {}", proxy_url);

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
    let proxy_url_test = env::var("SOCKS5_PROXY");

    match proxy_url_test {
        Ok(url) => {
            println!("[DEBUG] SOCKS5_PROXY env found: {}", url);
            info!("Request CLI client with proxy URL: {}", url);
        }
        Err(e) => {
            println!("[ERROR] Failed to read SOCKS5_PROXY: {}", e);
            panic!("[PANIC] SOCKS5_PROXY not set or invalid!"); // Ensure visibility of the issue
        }
    }

    let proxy_url =
        env::var("SOCKS5_PROXY").unwrap_or_else(|_| "socks5://your-proxy-ip:port".to_string());

    info!("Request CLI client with proxy URL: {}", proxy_url);

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
