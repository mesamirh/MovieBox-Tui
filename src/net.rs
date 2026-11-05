use std::net::SocketAddr;
use std::sync::Arc;

use hickory_resolver::TokioResolver;
use hickory_resolver::config::{
    CLOUDFLARE_IPS, GOOGLE_IPS, LookupIpStrategy, NameServerConfigGroup, QUAD9_IPS, ResolverConfig,
};
use hickory_resolver::name_server::TokioConnectionProvider;
use reqwest::dns::{Addrs, Name, Resolve, Resolving};

const FALLBACK_DNS_PORT: u16 = 53;
pub const DEFAULT_BROWSER_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";
pub const APP_HTTP_USER_AGENT: &str = concat!("MovieBox-Tui/", env!("CARGO_PKG_VERSION"));

static GLOBAL_RESOLVER: std::sync::LazyLock<Arc<TokioResolver>> =
    std::sync::LazyLock::new(|| Arc::new(build_resolver()));

static FALLBACK_RESOLVER: std::sync::LazyLock<Arc<TokioResolver>> =
    std::sync::LazyLock::new(|| Arc::new(build_fallback_resolver()));

fn build_fallback_resolver() -> TokioResolver {
    let mut builder =
        TokioResolver::builder_with_config(fallback_config(), TokioConnectionProvider::default());
    builder.options_mut().ip_strategy = LookupIpStrategy::Ipv4AndIpv6;
    builder.build()
}

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
        Err(e) => {
            log::warn!(
                "system DNS resolver failed ({e}); falling back to public DNS (Cloudflare/Google/Quad9)"
            );
            TokioResolver::builder_with_config(
                fallback_config(),
                TokioConnectionProvider::default(),
            )
        }
    };
    builder.options_mut().ip_strategy = LookupIpStrategy::Ipv4AndIpv6;
    builder.build()
}

impl Resolve for FallbackResolver {
    fn resolve(&self, name: Name) -> Resolving {
        let primary = Arc::clone(&GLOBAL_RESOLVER);
        let fallback = Arc::clone(&FALLBACK_RESOLVER);
        Box::pin(async move {
            match primary.lookup_ip(name.as_str()).await {
                Ok(lookup) => {
                    let addrs: Addrs = Box::new(
                        lookup
                            .into_iter()
                            .map(|address| SocketAddr::new(address, 0)),
                    );
                    Ok(addrs)
                }
                Err(primary_err) => {
                    log::debug!(
                        "primary DNS lookup for {} failed ({primary_err}); retrying with fallback public DNS",
                        name.as_str()
                    );
                    let lookup =
                        fallback
                            .lookup_ip(name.as_str())
                            .await
                            .map_err(|fallback_err| {
                                log::warn!(
                                    "fallback DNS lookup for {} also failed: {fallback_err}",
                                    name.as_str()
                                );
                                primary_err
                            })?;
                    let addrs: Addrs = Box::new(
                        lookup
                            .into_iter()
                            .map(|address| SocketAddr::new(address, 0)),
                    );
                    Ok(addrs)
                }
            }
        })
    }
}

pub fn http_client_builder_base() -> reqwest::ClientBuilder {
    reqwest::Client::builder()
        .dns_resolver(Arc::new(FallbackResolver::new()))
        .tcp_nodelay(true)
        .tcp_keepalive(Some(std::time::Duration::from_secs(45)))
        .pool_idle_timeout(Some(std::time::Duration::from_secs(90)))
        .pool_max_idle_per_host(8)
        .connect_timeout(std::time::Duration::from_secs(15))
}

pub fn http_client_builder() -> reqwest::ClientBuilder {
    http_client_builder_base().timeout(std::time::Duration::from_secs(60))
}

pub async fn probe_url(url: &str, timeout: std::time::Duration) -> bool {
    static PROBE_CLIENT: std::sync::LazyLock<reqwest::Client> = std::sync::LazyLock::new(|| {
        http_client_builder_base()
            .build()
            .unwrap_or_else(|_| reqwest::Client::new())
    });
    match tokio::time::timeout(timeout, PROBE_CLIENT.head(url).send()).await {
        Ok(Ok(resp)) if resp.status().is_success() || resp.status().is_redirection() => {
            return true;
        }
        Ok(Ok(resp)) => {
            log::debug!(
                "probe HEAD non-success for {}: {}",
                crate::logging::sanitize_url(url),
                resp.status()
            );
        }
        Ok(Err(e)) => {
            log::debug!(
                "probe HEAD error for {}: {e}",
                crate::logging::sanitize_url(url)
            );
        }
        Err(_) => {
            log::debug!(
                "probe HEAD timed out for {}",
                crate::logging::sanitize_url(url)
            );
        }
    }
    match tokio::time::timeout(
        timeout,
        PROBE_CLIENT
            .get(url)
            .header(reqwest::header::RANGE, "bytes=0-0")
            .send(),
    )
    .await
    {
        Ok(Ok(resp)) => resp.status().is_success() || resp.status().is_redirection(),
        Ok(Err(e)) => {
            log::debug!(
                "probe GET error for {}: {e}",
                crate::logging::sanitize_url(url)
            );
            false
        }
        Err(_) => false,
    }
}

pub fn is_http_url(source: &str) -> bool {
    let trimmed = source.trim();
    trimmed.starts_with("http://") || trimmed.starts_with("https://")
}

pub fn open_external_url(url: &str) -> std::io::Result<()> {
    if crate::config::is_test_environment() {
        return Ok(());
    }
    open::that(url)
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

    #[tokio::test]
    async fn probe_url_falls_back_from_head_to_ranged_get() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            for _ in 0..2 {
                if let Ok((mut stream, _)) = listener.accept().await {
                    let mut buf = [0u8; 1024];
                    let n = stream.read(&mut buf).await.unwrap_or(0);
                    let req = String::from_utf8_lossy(&buf[..n]);
                    if req.starts_with("HEAD ") {
                        let _ = stream
                            .write_all(
                                b"HTTP/1.1 405 Method Not Allowed\r\nContent-Length: 0\r\n\r\n",
                            )
                            .await;
                    } else if req.starts_with("GET ")
                        && req.to_ascii_lowercase().contains("range: bytes=0-0")
                    {
                        let _ = stream
                            .write_all(
                                b"HTTP/1.1 206 Partial Content\r\nContent-Length: 1\r\n\r\nX",
                            )
                            .await;
                    } else {
                        let _ = stream
                            .write_all(b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\n\r\n")
                            .await;
                    }
                }
            }
        });
        let url = format!("http://{addr}/video.mkv");
        assert!(probe_url(&url, std::time::Duration::from_secs(2)).await);
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
