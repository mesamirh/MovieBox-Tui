use std::net::SocketAddr;
use std::sync::Arc;

use hickory_resolver::TokioResolver;
use hickory_resolver::config::{
    CLOUDFLARE_IPS, GOOGLE_IPS, LookupIpStrategy, NameServerConfigGroup, QUAD9_IPS, ResolverConfig,
};
use hickory_resolver::name_server::TokioConnectionProvider;
use reqwest::dns::{Addrs, Name, Resolve, Resolving};

const FALLBACK_DNS_PORT: u16 = 53;

static GLOBAL_RESOLVER: std::sync::LazyLock<Arc<TokioResolver>> =
    std::sync::LazyLock::new(|| Arc::new(build_resolver()));

#[derive(Debug, Default, Clone)]
pub struct FallbackResolver;

impl FallbackResolver {
    pub fn new() -> Self {
        Self
    }
}

fn fallback_servers() -> Vec<std::net::IpAddr> {
    let mut servers = Vec::with_capacity(CLOUDFLARE_IPS.len() + GOOGLE_IPS.len() + QUAD9_IPS.len());
    servers.extend_from_slice(CLOUDFLARE_IPS);
    servers.extend_from_slice(GOOGLE_IPS);
    servers.extend_from_slice(QUAD9_IPS);
    servers
}

fn fallback_config() -> ResolverConfig {
    ResolverConfig::from_parts(
        None,
        Vec::new(),
        NameServerConfigGroup::from_ips_clear(&fallback_servers(), FALLBACK_DNS_PORT, true),
    )
}

fn build_resolver() -> TokioResolver {
    let mut builder = match TokioResolver::builder_tokio() {
        Ok(builder) => builder,
        Err(_) => TokioResolver::builder_with_config(
            fallback_config(),
            TokioConnectionProvider::default(),
        ),
    };
    builder.options_mut().ip_strategy = LookupIpStrategy::Ipv4AndIpv6;
    builder.build()
}

impl Resolve for FallbackResolver {
    fn resolve(&self, name: Name) -> Resolving {
        let resolver = Arc::clone(&GLOBAL_RESOLVER);
        Box::pin(async move {
            let lookup = resolver.lookup_ip(name.as_str()).await?;
            let addrs: Addrs = Box::new(
                lookup
                    .into_iter()
                    .map(|address| SocketAddr::new(address, 0)),
            );
            Ok(addrs)
        })
    }
}

pub fn http_client_builder() -> reqwest::ClientBuilder {
    reqwest::Client::builder()
        .dns_resolver(Arc::new(FallbackResolver::new()))
        .tcp_nodelay(true)
        .tcp_keepalive(Some(std::time::Duration::from_secs(45)))
        .pool_idle_timeout(Some(std::time::Duration::from_secs(90)))
        .pool_max_idle_per_host(8)
}

pub async fn probe_url(url: &str, timeout: std::time::Duration) -> bool {
    let Ok(client) = reqwest::Client::builder()
        .timeout(timeout)
        .connect_timeout(timeout)
        .build()
    else {
        return false;
    };
    if client.head(url).send().await.is_ok() {
        return true;
    }
    client.get(url).send().await.is_ok()
}

pub fn is_http_url(source: &str) -> bool {
    let trimmed = source.trim();
    trimmed.starts_with("http://") || trimmed.starts_with("https://")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_config_contains_public_resolvers() {
        let config = fallback_config();
        let servers = config.name_servers();
        assert!(servers.len() >= 12);
        assert!(servers.iter().any(|server| server.socket_addr.ip()
            == std::net::IpAddr::V4(std::net::Ipv4Addr::new(1, 1, 1, 1))));
        assert!(
            servers
                .iter()
                .any(|server| server.socket_addr.port() == FALLBACK_DNS_PORT)
        );
    }

    #[test]
    fn fallback_servers_deduplicate_nothing_and_cover_all_providers() {
        let servers = fallback_servers();
        assert_eq!(
            servers.len(),
            CLOUDFLARE_IPS.len() + GOOGLE_IPS.len() + QUAD9_IPS.len()
        );
    }

    #[tokio::test]
    async fn built_resolver_prefers_ipv4_and_ipv6_lookup() {
        let resolver = build_resolver();
        assert_eq!(
            resolver.options().ip_strategy,
            LookupIpStrategy::Ipv4AndIpv6
        );
    }

    #[test]
    fn clones_share_lazy_state_slot() {
        let original = FallbackResolver::new();
        let _clone = original.clone();
        let _builder = http_client_builder();
    }
    #[test]
    fn test_is_http_url() {
        assert!(is_http_url("http://example.com"));
        assert!(is_http_url("https://example.com/playlist.m3u8"));
        assert!(is_http_url("   https://example.com   "));
        assert!(!is_http_url("/local/path/file.m3u"));
        assert!(!is_http_url("stremio://addon.example.com"));
        assert!(!is_http_url(""));
    }
}
