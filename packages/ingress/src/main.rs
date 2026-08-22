use c_plane_ingress::{config::Config, proxy::Ingress};
use pingora::{prelude::*, proxy::http_proxy_service};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let config =
        Config::from_env().map_err(|error| format!("invalid ingress configuration: {error}"))?;
    let listen = config.listen.clone();
    let management_listen =
        std::env::var("INGRESS_MANAGEMENT_LISTEN").unwrap_or_else(|_| "0.0.0.0:9090".to_owned());

    let mut server = Server::new(Some(Opt::parse_args()))?;
    server.bootstrap();

    let mut proxy = http_proxy_service(&server.configuration, Ingress::new(config));
    for address in &listen {
        proxy.add_tcp(address);
    }
    server.add_service(proxy);

    if !management_listen.trim().is_empty() {
        let mut metrics = pingora::services::listening::Service::prometheus_http_service();
        metrics.add_tcp(&management_listen);
        server.add_service(metrics);
    }

    tracing::info!(?listen, management_listen, "ingress starting");
    server.run_forever();
}
