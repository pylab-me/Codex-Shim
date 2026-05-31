use std::net::SocketAddr;

use net_chassis::socket::bind_tcp_listener;
use net_chassis::tuning::{Ipv6BindMode, TcpIngressOptions};
use tracing::info;

fn main() -> anyhow::Result<()> {
    if let Some(exit_code) = codex_shim::handle_process_metadata_command() {
        std::process::exit(exit_code);
    }

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async_main())
}

async fn async_main() -> anyhow::Result<()> {
    let config = codex_shim::AppConfig::load()?;
    codex_shim::init_tracing(&config.log_level);

    let bind_addr = config.bind;
    let state = codex_shim::create_state(config).await?;

    let app = codex_shim::create_router(state).into_make_service_with_connect_info::<SocketAddr>();

    // Use net-chassis for socket creation — gets SocketMeta for free
    let mut tcp_options = TcpIngressOptions::baseline(0, Ipv6BindMode::DualStack);
    tcp_options.listen_backlog = 1024;
    tcp_options.accept_channel_capacity = 1024;
    tcp_options.reuseaddr = true;

    let (listener, _socket_meta) = bind_tcp_listener(bind_addr, &tcp_options)?;
    info!(bind = %bind_addr, "codex-shim started");
    axum::serve(listener, app).await?;
    Ok(())
}
