# Streaming Providers

MovieBox-TUI searches and streams media across multiple independent providers.

## Available Providers

| Provider | Description |
| :--- | :--- |
| **MovieBox** | Primary streaming catalog with multi-language audio, seasons, and subtitles. |

Subtitle tracks come from the provider itself. When a release has no usable
captions, the core can also search OpenSubtitles online: it resolves the title's
IMDb id through the public suggestion endpoint, queries the keyless
`opensubtitles-v3.strem.io` addon and returns the matches ordered Spanish, English,
then everything else. The search never fails playback — an unreachable service or
an unresolved title simply yields no online entries.
| **4KHDHub** | High-bitrate 4K UHD and 1080p releases with fast CDN mirrors. |
| **Dramachi** | Asian dramas and series catalog. |
| **CircleFTP** | High-speed local BDIX mirror (Bangladesh ISPs). |
| **DhakaFlix** | Local BDIX media indexer (Bangladesh ISPs). |
| **Addons** | Community Stremio HTTP addons (Cinemeta catalog and streams). |

## Switching Providers

- **Cycle Provider**: Press **`Ctrl+P`** on the home screen to switch to the next provider.
- **Provider Menu**: Click the provider badge in the search bar (or press `Enter` on it) to open the provider selection menu.
- **Enable / Disable Providers**: Open `/settings` → **Content Modes** → **Streaming Sources** to toggle providers on or off.

## BDIX Network Detection

If you are connected through a Bangladeshi ISP supporting BDIX:
- MovieBox-TUI automatically tests local BDIX mirrors on startup and enables them if reachable.
- You can manually re-test your connection anytime via `/settings` → **Maintenance** → **Re-check BDIX Network**.

## Provider Architecture (For Developers)

All providers implement a shared Rust trait in `src/providers/`:

```rust
pub trait Provider: Send + Sync {
    fn id(&self) -> ProviderKind;
    fn capabilities(&self) -> ProviderCapabilities;
    async fn search(&self, query: &str, page: usize) -> Result<Vec<CatalogItem>, ProviderError>;
    async fn details(&self, id: &str) -> Result<MediaDetails, ProviderError>;
}
```

New providers implement `Provider` (and optionally `ReleaseProvider` for multi-mirror streams), then register in `src/service.rs`. Responses are automatically cached as MessagePack envelopes.

### MovieBox CDN Manifest & Failover Resolution

MovieBox CDN stream entries parse signed `CloudFront-Policy` and `Edge-Cache-Cookie` tokens to find stream roots. The manifest resolver accepts only `/dash/` resources. Unhosted legacy `/hls/` cookie targets are dropped to prevent 404 failures during playback or downloads.

When upstream APIs return placeholder deprecation notices, MovieBox-TUI automatically queries the `/resource-position/v2` failover endpoint with scoped episode resolution to recover genuine signed MP4 streams, omitting the DASH Referer to prevent CDN HTTP 429 throttling.
