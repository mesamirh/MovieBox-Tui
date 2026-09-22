<div align="center">

# MovieBox-TUI

**App de terminal para buscar, descargar y ver películas, series y TV en vivo con tus reproductores locales favoritos.**

[ English ](README.md) • [ বাংলা ](README_BN.md) • [ हिन्दी ](README_HI.md) • [ Español ](README_ES.md)

[![Telegram](https://telegram-badge.vercel.app/api/telegram-badge?channelId=@getfromme&style=flat&logo=true)](https://t.me/getfromme)
[![Donate](https://img.shields.io/badge/Donate-Crypto-F7931A?style=flat&logo=bitcoin&logoColor=white)](#optional-support)
</div>

[moviebox-tui-walkthrough.webm](https://github.com/user-attachments/assets/7554a7e5-6ff5-49ec-9d87-f821ea99950e)

## Qué incluye

- **Películas y series en streaming**: Disfruta de películas, series y anime desde MovieBox, 4KHDHub, servidores BDIX y complementos de Stremio.
- **TV en vivo e IPTV**: Agrega tus listas M3U para buscar canales, explorar categorías y ver televisión en directo.
- **Reproducción fluida**: Abre el video directamente en tu reproductor favorito (`mpv`, `IINA`, `VLC` o reproductores en Android) sin navegadores pesados y con aceleración por hardware.
- **Subtítulos automáticos**: Encuentra y carga subtítulos en tu idioma automáticamente dentro del reproductor.
- **Descargas fáciles**: Guarda episodios o temporadas completas en tu equipo con soporte para pausar y reanudar.
- **Control de calidad**: Elige una resolución preferida una sola vez y se aplicará a cada reproducción y descarga, ideal para ahorrar datos en conexiones lentas.
- **Pósters y carátulas**: Muestra carátulas y pósters a color directamente en tu ventana de terminal.
- **Historial y favoritos**: Guarda lo que te gusta en marcadores y retoma la reproducción exactamente donde la dejaste.
- **Temas integrados**: 6 temas de color incluidos para combinar con el estilo de tu terminal.
- **Multiplataforma**: Funciona a la perfección en macOS, Linux, Windows y Android (Termux).

## Lo que necesitas

Solo necesitas tener instalado al menos uno de estos reproductores:

- **mpv** (la mejor opción en Linux, macOS y Windows)
- **IINA** (para macOS)
- **VLC** (para cualquier sistema)
- **Cualquier reproductor en Android** vía Termux (VLC, Just Player, MX Player)

*Para ver pósters:* Requiere una terminal moderna con soporte gráfico (Ghostty, Kitty, WezTerm o iTerm2). Las terminales normales mostrarán el contenido en texto limpio.

*Para descargas de MovieBox:* Solo las descargas desde MovieBox requieren `yt-dlp` y `ffmpeg`. Los demás proveedores descargan directamente sin herramientas externas.

## Cómo instalarlo

### macOS y Linux

Instalación automática:
```bash
curl -fsSL https://raw.githubusercontent.com/mesamirh/MovieBox-Tui/main/install.sh | bash
```

Con Homebrew (macOS):
```bash
brew tap mesamirh/moviebox-tui https://github.com/mesamirh/MovieBox-Tui
brew install moviebox-tui
```
*Nota:* Si Homebrew te pide confirmación, ejecuta `brew trust mesamirh/moviebox-tui`.

### Windows

Desde PowerShell:
```powershell
irm https://raw.githubusercontent.com/mesamirh/MovieBox-Tui/main/install.ps1 | iex
```

### Android (Termux)

1. Instala las dependencias y ejecuta el instalador:
```bash
pkg update && pkg install -y curl tar termux-tools termux-am
curl -fsSL https://raw.githubusercontent.com/mesamirh/MovieBox-Tui/main/install.sh | bash
```

2. Da permisos de almacenamiento (necesario para que el reproductor abra los videos y subtítulos):
```bash
termux-setup-storage
```

<details>
<summary><b>Instalar con Cargo o compilar el código fuente</b></summary>

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
<summary><b>Verificar la integridad del archivo (Checksum)</b></summary>

```bash
sha256sum -c SHA256SUMS --ignore-missing
gh attestation verify <archive-file> -R mesamirh/MovieBox-Tui
```

</details>

## Cómo empezar

```bash
moviebox-tui
```

- Escribe el nombre de cualquier película o serie para buscar, y presiona `Enter` para reproducir.
- Presiona `?` dentro de la aplicación para ver los atajos, o escribe `/settings` para abrir la configuración.

## Documentación

Encuentra guías completas y detalles de arquitectura en [**mesamirh.github.io/MovieBox-Tui**](https://mesamirh.github.io/MovieBox-Tui/) o en la carpeta [`docs/`](docs/).

## Contribuir

¡Cualquier aporte es bienvenido! Por favor revisa [CONTRIBUTING.md](CONTRIBUTING.md) antes de enviar un pull request.

Si encuentras un fallo o tienes una idea para mejorar, cuéntanos en [GitHub Issues](https://github.com/mesamirh/MovieBox-Tui/issues).

<details>
<summary><b>Apoyar el desarrollo</b></summary>
<div id="optional-support" tabindex="-1"></div>

Si deseas apoyar el desarrollo directamente:

| Red / Criptomoneda | Dirección |
| :--- | :--- |
| **USDT (TRC20)** | `TL4yW73qmbKZpBWwbEFgjBpwVkPDFTkJgV` |
| **Bitcoin (BTC)** | `3MEAtqtRWrQBhnaMi3Zuf5nt2efNUS2LUQ` |
| **Ethereum / EVM** | `0x7ea20d5fa29d87f33195f5a3b211ff94038d794c` |
| **Solana (SOL)** | `6ctm5WFv73MNywoCKAz3xK72yizSspHa72rFNygooU6` |

</details>

## Privacidad y seguridad

MovieBox-TUI no incluye telemetría, analíticas ni rastreo de ningún tipo. Todo tu historial de búsqueda, marcadores y configuración se guardan exclusivamente en tu propio equipo.

## Licencia

Distribuido bajo la licencia [MIT](LICENSE-MIT) o [Apache-2.0](LICENSE-APACHE).

## Aviso legal

Este proyecto no aloja ni almacena ningún archivo multimedia. Es únicamente un cliente de terminal de código abierto para reproducir enlaces públicos de video. Cada usuario es responsable de cumplir con las leyes de su país.
