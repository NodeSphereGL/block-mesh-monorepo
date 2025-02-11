use crate::constants::DeviceType;
use reqwest::{Client, ClientBuilder, Proxy};
#[allow(unused_imports)]
use std::{env, time::Duration};

#[cfg(target_arch = "wasm32")]
pub fn http_client(device_type: DeviceType) -> Client {
    let mut client_builder = ClientBuilder::new().user_agent(format!(
        "curl/8.7.1; {}; {}",
        device_type,
        env!("CARGO_PKG_VERSION")
    ));

    // Add proxy support (SOCKS5)
    if let Ok(proxy_url) = env::var("SOCKS5_PROXY") {
        match Proxy::all(&proxy_url) {
            Ok(proxy) => {
                client_builder = client_builder.proxy(proxy);
                println!("Using SOCKS5 proxy: {}", proxy_url); // Add logging
            }
            Err(e) => {
                eprintln!("Failed to create SOCKS5 proxy: {}", e); // Add error logging
            }
        }
    }

    client_builder.build().unwrap_or_default()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn http_client(device_type: DeviceType) -> Client {
    let mut client_builder = ClientBuilder::new()
        .timeout(Duration::from_secs(3))
        .cookie_store(true)
        .user_agent(format!(
            "curl/8.7.1; {}; {}",
            device_type,
            env!("CARGO_PKG_VERSION")
        ))
        .no_hickory_dns()
        .use_rustls_tls();

    // Add proxy support (SOCKS5)
    if let Ok(proxy_url) = env::var("SOCKS5_PROXY") {
        match Proxy::all(&proxy_url) {
            Ok(proxy) => {
                client_builder = client_builder.proxy(proxy);
                println!("Using SOCKS5 proxy: {}", proxy_url); // Add logging
            }
            Err(e) => {
                eprintln!("Failed to create SOCKS5 proxy: {}", e); // Add error logging
            }
        }
    }

    client_builder.build().unwrap_or_default()
}
