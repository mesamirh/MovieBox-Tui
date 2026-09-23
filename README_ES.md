<div align="center">

# MovieBox-TUI

**Interfaz de terminal para buscar, descargar y transmitir películas, series y TV en vivo mediante reproductores locales.**

[ English ](README.md) • [ বাংলা ](README_BN.md) • [ हिन्दी ](README_HI.md) • [ Español ](README_ES.md)

[![CI](https://img.shields.io/github/actions/workflow/status/mesamirh/MovieBox-Tui/ci.yml?branch=main&label=CI&logo=github&style=flat)](https://github.com/mesamirh/MovieBox-Tui/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/moviebox-tui.svg?logo=rust&style=flat)](https://crates.io/crates/moviebox-tui)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg?style=flat)](#licencia)
[![Telegram](https://img.shields.io/badge/Telegram-Channel-2CA5E0?style=flat&logo=telegram&logoColor=white)](https://t.me/getfromme)
[![Support](https://img.shields.io/badge/Support-Crypto-F7931A?style=flat&logo=bitcoin&logoColor=white)](#apoyo-opcional)
</div>

[moviebox-tui-walkthrough.webm](https://github.com/user-attachments/assets/7554a7e5-6ff5-49ec-9d87-f821ea99950e)

## Características

- **Streaming bajo demanda**: Reproduce películas, series, anime y dramas asiáticos a través de múltiples proveedores nativos y complementos comunitarios de Stremio.
- **TV en vivo e IPTV**: Importa listas M3U personalizadas para buscar canales, explorar categorías y ver televisión en directo.
- **Selección de calidad y resolución**: Elige la resolución deseada (`1080p`, `720p`, `480p`) directamente en la pantalla de detalles.
- **Reproducción acelerada por hardware**: Se ejecuta directamente en tu reproductor multimedia local favorito con autenticación y envío automático de cookies.
- **Descargador multisegmento**: Descarga episodios individuales o temporadas completas en paralelo, con soporte para pausar y reanudar por rangos HTTP.
- **Subtítulos automáticos**: Busca y sincroniza subtítulos en tu idioma preferido de forma automática.
- **Interfaz de terminal interactiva**: Soporte completo para teclado y ratón, con navegación estilo vim y sugerencias automáticas de comandos.
- **Pósters visuales y temas**: Renderiza carátulas directamente en tu terminal, con 9 temas integrados y detección automática de modo claro y oscuro.
- **Biblioteca y seguimiento de progreso**: Marca favoritos, registra el historial de visualización y reanuda la reproducción exactamente donde la dejaste.
- **Multiplataforma y privado**: Funciona de forma nativa en macOS, Linux, Windows y Android (Termux) sin telemetría ni recopilación de datos.

## Requisitos previos

### Reproductores multimedia compatibles

Requiere tener instalado al menos un reproductor multimedia compatible en tu sistema:

- **Escritorio (macOS, Linux, Windows):** [mpv](https://mpv.io/), [VLC](https://www.videolan.org/), o [IINA](https://iina.io/) *(macOS)*.
- **Android (Termux):** Cualquier reproductor de video externo ([VLC](https://play.google.com/store/apps/details?id=org.videolan.vlc), Just Player, o MX Player).

### Gráficos en la terminal (Pósters)

La visualización de pósters se adapta automáticamente a tu terminal:

- **Terminales con soporte gráfico:** Muestra pósters de películas y series en alta resolución de forma nativa.
- **Terminales estándar:** Muestra marcadores de posición de texto limpios y estructurados.

### Dependencias opcionales

- **`yt-dlp` y `ffmpeg`:** Requeridos únicamente al descargar transmisiones DASH del proveedor MovieBox. El resto de proveedores descargan directamente a través del motor HTTP integrado.

## Instalación

### macOS y Linux

Abre la terminal y ejecuta:
```bash
curl -fsSL https://raw.githubusercontent.com/mesamirh/MovieBox-Tui/main/install.sh | bash
```

O mediante Homebrew (macOS):
```bash
brew tap mesamirh/moviebox-tui https://github.com/mesamirh/MovieBox-Tui
brew install moviebox-tui
```
> [!NOTE]
> Si Homebrew solicita verificación de tap en la instalación inicial, ejecuta `brew trust mesamirh/moviebox-tui`.

### Windows

A través de Scoop:
```powershell
scoop bucket add moviebox https://github.com/mesamirh/MovieBox-Tui
scoop install moviebox-tui
```

O mediante el script automatizado de PowerShell:
```powershell
irm https://raw.githubusercontent.com/mesamirh/MovieBox-Tui/main/install.ps1 | iex
```

### Android (Termux)

Abre Termux y ejecuta:
```bash
pkg update && pkg install -y curl tar termux-tools termux-am
curl -fsSL https://raw.githubusercontent.com/mesamirh/MovieBox-Tui/main/install.sh | bash
termux-setup-storage
```
> [!IMPORTANT]
> La reproducción de video en Android se abre mediante tu reproductor externo instalado (como VLC, Just Player, o MX Player).

<details>
<summary><b>Instalación con Cargo o compilación desde el código fuente</b></summary>

Desde crates.io:
```bash
cargo install moviebox-tui --locked
```

Compilar desde el código fuente:
```bash
git clone https://github.com/mesamirh/MovieBox-Tui.git
cd MovieBox-Tui
cargo build --release --locked
```

</details>

<details>
<summary><b>Verificación de integridad de la versión</b></summary>

```bash
sha256sum -c SHA256SUMS --ignore-missing
gh attestation verify <archive-file> -R mesamirh/MovieBox-Tui
```

</details>

<details>
<summary><b>Desinstalación</b></summary>

#### Instalador automatizado (macOS, Linux, Windows, Android)

Simplemente vuelve a ejecutar el comando de instalación original (`curl ... | bash` o `irm ... | iex`). Cuando MovieBox-TUI ya está instalado, el instalador lo detecta y muestra un menú interactivo:

```text
MovieBox-TUI is already installed.
What would you like to do?
  1) Reinstall / Update to latest version
  2) Uninstall
  3) Cancel
```

Introduce `2` para eliminar completamente MovieBox-TUI.

#### Gestores de paquetes

```bash
brew uninstall moviebox-tui     # Homebrew (macOS)
scoop uninstall moviebox-tui    # Scoop (Windows)
cargo uninstall moviebox-tui    # Cargo
```

</details>

## Inicio rápido

```bash
moviebox-tui
```

- Escribe cualquier título para buscar, presiona `Enter` para reproducir.
- Presiona `?` dentro de la interfaz para ver los atajos, o escribe `/settings` para abrir las preferencias.

## Documentación

Las guías completas y referencias de arquitectura están disponibles en [**mesamirh.github.io/MovieBox-Tui**](https://mesamirh.github.io/MovieBox-Tui/) o en el directorio [`docs/`](docs/):

| Guía | Descripción |
| :--- | :--- |
| [Teclado y Controles](docs/controls.md) | Atajos de teclado, navegación vim, búsqueda y comandos |
| [Configuración](docs/config.md) | Opciones de configuración, personalización de temas y variables de entorno |
| [Proveedores de Contenido](docs/providers.md) | Scrapers nativos (MovieBox, Dramachi, BDIX) y extractores de stream |
| [Complementos de Stremio](docs/addons-mode.md) | Instalación de addons comunitarios, URLs de manifiestos y resolución de streams |
| [Reproductores Multimedia](docs/players.md) | Detección de reproductores, opciones de inicio y aceleración por hardware |
| [TV en vivo e IPTV](docs/tv-mode.md) | Importación de listas M3U, búsqueda de canales y streaming en vivo |
| [Descargas por Lotes](docs/downloads.md) | Descargas multisegmento por rangos HTTP con soporte de pausa y reanudación |

## Contribuir

Cualquier contribución es bienvenida. Revisa [CONTRIBUTING.md](CONTRIBUTING.md) antes de enviar pull requests.

Reporta errores o sugiere nuevas funciones a través de [GitHub Issues](https://github.com/mesamirh/MovieBox-Tui/issues).

<details>
<summary><b>Apoyo opcional</b></summary>
<div id="apoyo-opcional" tabindex="-1"></div>

Si deseas apoyar el desarrollo continuo directamente:

| Red / Criptomoneda | Dirección |
| :--- | :--- |
| **USDT (TRC20)** | `TL4yW73qmbKZpBWwbEFgjBpwVkPDFTkJgV` |
| **Bitcoin (BTC)** | `3MEAtqtRWrQBhnaMi3Zuf5nt2efNUS2LUQ` |
| **Ethereum / EVM** | `0x7ea20d5fa29d87f33195f5a3b211ff94038d794c` |
| **Solana (SOL)** | `6ctm5WFv73MNywoCKAz3xK72yizSspHa72rFNygooU6` |

</details>

## Privacidad

MovieBox-TUI contiene cero telemetría, análisis o seguimiento de usuarios. Todo el historial de búsqueda, marcadores y archivos de configuración permanecen estrictamente en tu sistema de archivos local.

## Licencia

Distribuido bajo licencias [MIT](LICENSE-MIT) o [Apache-2.0](LICENSE-APACHE).

## Descargo de responsabilidad

Este proyecto no aloja ni almacena ningún contenido multimedia. Es un cliente independiente para reproducir transmisiones de video disponibles públicamente. Los usuarios son responsables de cumplir con las leyes de sus respectivos países.
