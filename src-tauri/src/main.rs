#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::net::SocketAddr;

use net_chassis::socket::bind_tcp_listener;
use net_chassis::tuning::{TcpIngressOptions, Ipv6BindMode};
use net_chassis::cancellation::CancellationToken;
use tracing::error;
use tracing::info;

#[tokio::main]
async fn main() {
    // 1. 优先处理元数据命令
    if let Some(exit_code) = codex_shim::handle_process_metadata_command() {
        std::process::exit(exit_code);
    }

    // 2. 生产标准：在主上下文中初始化配置与日志，确保任何前置错误能被捕获
    let config = codex_shim::AppConfig::load().expect("failed to load config");
    codex_shim::init_tracing(&config.log_level);

    let bind_addr = config.bind;
    let state = codex_shim::create_state(config).await.expect("failed to create state");

    let app = codex_shim::create_router(state).into_make_service_with_connect_info::<SocketAddr>();

    // 3. Use net-chassis for socket creation — production-grade socket tuning
    let mut tcp_options = TcpIngressOptions::baseline(0, Ipv6BindMode::DualStack);
    tcp_options.listen_backlog = 1024;
    tcp_options.accept_channel_capacity = 1024;
    tcp_options.reuseaddr = true;

    let (listener, _socket_meta) = match bind_tcp_listener(bind_addr, &tcp_options) {
        Ok(result) => result,
        Err(e) => {
            error!(error = %e, address = %bind_addr, "Failed to bind HTTP server port");
            std::process::exit(1);
        }
    };
    let addr = listener.local_addr().unwrap_or(bind_addr);

    // 4. Graceful shutdown via CancellationToken
    let shutdown_token = CancellationToken::new();
    let shutdown_token_clone = shutdown_token.clone();

    // 5. Start Axum with graceful shutdown
    let server_handle = tokio::spawn(async move {
        info!(bind = %addr, "HTTP API server started for Tauri (net-chassis)");

        let server = axum::serve(listener, app).with_graceful_shutdown(async move {
            shutdown_token_clone.cancelled().await;
            info!("Axum backend received shutdown signal, closing gracefully...");
        });

        if let Err(e) = server.await {
            error!(error = %e, "Axum server encountered an error");
        }
    });

    // 6. Run Tauri desktop window
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .on_window_event(move |_window, event| {
            if let tauri::WindowEvent::Destroyed = event {
                info!("Tauri window destroyed, triggering graceful shutdown");
                shutdown_token.cancel();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

    // Wait for server to finish
    let _ = server_handle.await;
}