# Installation

MovieBox-TUI is available across macOS, Linux, Windows, and Android (Termux).

---

## macOS and Linux

### Homebrew (macOS)

If you have [Homebrew](https://brew.sh/):

```bash
brew tap mesamirh/moviebox-tui https://github.com/mesamirh/MovieBox-Tui
brew install moviebox-tui
```

> [!NOTE]
> If Homebrew prompts for tap verification on initial install, run `brew trust mesamirh/moviebox-tui`.

### Direct Terminal Script

No package manager required. Open Terminal and run:

```bash
curl -fsSL https://raw.githubusercontent.com/mesamirh/MovieBox-Tui/main/install.sh | bash
```

---

## Windows

### Scoop Package Manager (Recommended)

If you have [Scoop](https://scoop.sh/):

```powershell
scoop bucket add moviebox https://github.com/mesamirh/MovieBox-Tui
scoop install moviebox-tui
```

### Direct PowerShell Script

No package manager required. Open **PowerShell** and run:

```powershell
irm https://raw.githubusercontent.com/mesamirh/MovieBox-Tui/main/install.ps1 | iex
```

> **SmartScreen prompt:** If Windows displays *"Windows protected your PC"*, click **More info** → **Run anyway**.
---

## Android (Termux)

Open Termux and run:

```bash
pkg update && pkg install -y curl tar termux-tools termux-am
curl -fsSL https://raw.githubusercontent.com/mesamirh/MovieBox-Tui/main/install.sh | bash
termux-setup-storage
```

> [!IMPORTANT]
> Video playback on Android launches through your installed external media player (such as VLC, Just Player, or MX Player).

> [!NOTE]
> Android 5.0–5.1.1 (e.g. Oppo A37fw, `aarch64 / sdk 22`): use [Android 5.x Legacy Install](android-legacy.md). Download to `$HOME` (not `/tmp`), and use the API-21 `DT_HASH` build until a post-fix release is published.
---

## Cargo (Crates.io)

Install directly using Cargo:

```bash
cargo install moviebox-tui --locked
```

---

## Compile from Source

Clone the repository and build the release binary:

```bash
git clone https://github.com/mesamirh/MovieBox-Tui.git
cd MovieBox-Tui
cargo build --release --locked
```

The compiled binary will be located at `target/release/moviebox-tui`.

---

## Verify Release Integrity

All release assets include cryptographically signed SHA-256 checksums and GitHub provenance attestations:

```bash
sha256sum -c SHA256SUMS --ignore-missing
gh attestation verify <archive-file> -R mesamirh/MovieBox-Tui
```
---

## Uninstallation

### Automated Installer (macOS, Linux, Windows, Android)

Simply re-run your original install command (`curl ... | bash` or `irm ... | iex`). When MovieBox-TUI is already installed, the installer automatically detects it and displays an interactive menu:

```text
MovieBox-TUI is already installed.
What would you like to do?
  1) Reinstall / Update to latest version
  2) Uninstall
  3) Cancel
```

Enter `2` to completely remove MovieBox-TUI from your system.

### Package Managers

```bash
brew uninstall moviebox-tui     # Homebrew (macOS)
scoop uninstall moviebox-tui    # Scoop (Windows)
cargo uninstall moviebox-tui    # Cargo
```
