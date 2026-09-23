<div align="center">

# MovieBox-TUI

**स्थानीय मीडिया प्लेयर से फिल्में, टीवी शो और लाइव टीवी सर्च करने, डाउनलोड और स्ट्रीम करने का टर्मिनल इंटरफेस।**

[ English ](README.md) • [ বাংলা ](README_BN.md) • [ हिन्दी ](README_HI.md) • [ Español ](README_ES.md)

[![CI](https://img.shields.io/github/actions/workflow/status/mesamirh/MovieBox-Tui/ci.yml?branch=main&label=CI&logo=github&style=flat)](https://github.com/mesamirh/MovieBox-Tui/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/moviebox-tui.svg?logo=rust&style=flat)](https://crates.io/crates/moviebox-tui)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg?style=flat)](#license)
[![Telegram](https://img.shields.io/badge/Telegram-Channel-2CA5E0?style=flat&logo=telegram&logoColor=white)](https://t.me/getfromme)
[![Support](https://img.shields.io/badge/Support-Crypto-F7931A?style=flat&logo=bitcoin&logoColor=white)](#optional-support)
</div>

[moviebox-tui-walkthrough.webm](https://github.com/user-attachments/assets/7554a7e5-6ff5-49ec-9d87-f821ea99950e)

## खास फीचर्स

- **ऑन-डिमांड स्ट्रीमिंग**: कई नेटिव प्रोवाइडर्स और कम्युनिटी Stremio ऐड-ऑन्स से फिल्में, टीवी सीरीज, एनीमे और एशियन ड्रामा स्ट्रीम करें।
- **लाइव टीवी और IPTV**: चैनल्स सर्च करने, कैटेगरी ब्राउज करने और लाइव टेलीविजन स्ट्रीम करने के लिए कस्टम M3U प्लेलिस्ट URL इम्पोर्ट करें।
- **क्वालिटी और रेजोल्यूशन चयन**: डिटेल्स स्क्रीन से सीधे अपनी पसंदीदा स्ट्रीम रेजोल्यूशन (`1080p`, `720p`, `480p`) चुनें।
- **हार्डवेयर त्वरित प्लेबैक**: ऑटोमैटिक ऑथेंटिकेशन और कुकी फॉरवर्डिंग के साथ सीधे अपने पसंदीदा स्थानीय मीडिया प्लेयर में चलाएं।
- **मल्टी-सेगमेंट डाउनलोडर**: HTTP रेंज पॉज और रिज्यूम सपोर्ट के साथ व्यक्तिगत एपिसोड या पूरे सीजन एक साथ डाउनलोड करें।
- **ऑटोमैटिक सबटाइटल**: आपकी पसंदीदा भाषा में सबटाइटल अपने आप खोजकर सिंक्रोनाइज़ करता है।
- **इंटरैक्टिव टर्मिनल UI**: vim नेविगेशन और कमांड ऑटो-सजेशन के साथ फुल कीबोर्ड और माउस सपोर्ट।
- **विज़ुअल पोस्टर और थीम्स**: 9 इन-बिल्ट थीम्स और ऑटोमैटिक लाइट/डार्क डिटेक्शन के साथ सीधे टर्मिनल में कवर आर्ट रेंडर करता है।
- **लाइब्रेरी और प्रोग्रेस ट्रैकिंग**: पसंदीदा स्टार करें, वॉच हिस्ट्री ट्रैक करें और वहीं से शुरू करें जहां आपने छोड़ा था।
- **क्रॉस-प्लेटफॉर्म और प्राइवेट**: बिना किसी टेलीमेट्री या डेटा कलेक्शन के macOS, Linux, Windows और Android (Termux) पर नेटिव रूप से चलता है।

## पूर्व-आवश्यकताएं

### समर्थित मीडिया प्लेयर

आपके सिस्टम पर कम से कम एक समर्थित मीडिया प्लेयर इंस्टॉल होना आवश्यक है:

- **डेस्कटॉप (macOS, Linux, Windows):** [mpv](https://mpv.io/), [VLC](https://www.videolan.org/), या [IINA](https://iina.io/) *(macOS)*।
- **एंड्रॉइड (Termux):** कोई भी बाहरी वीडियो प्लेयर ([VLC](https://play.google.com/store/apps/details?id=org.videolan.vlc), Just Player, या MX Player)।

### टर्मिनल ग्राफिक्स (पोस्टर)

पोस्टर रेंडरिंग आपके टर्मिनल वातावरण के अनुसार अपने आप अनुकूलित हो जाती है:

- **ग्राफिक्स-सक्षम टर्मिनल:** हाई-रेजोल्यूशन मूवी और सीरीज पोस्टर नेटिव रूप से दिखाता है।
- **सामान्य टर्मिनल:** साफ और स्ट्रक्चर्ड टेक्स्ट प्लेसहोल्डर अपने आप दिखाता है।

### वैकल्पिक निर्भरताएं

- **`yt-dlp` और `ffmpeg`:** केवल MovieBox प्रोवाइडर से DASH स्ट्रीम्स डाउनलोड करने के लिए आवश्यक। बाकी सभी प्रोवाइडर्स सीधे इन-बिल्ट HTTP इंजन से डाउनलोड होते हैं।

## इंस्टॉलेशन

### macOS और Linux

टर्मिनल खोलें और चलाएं:
```bash
curl -fsSL https://raw.githubusercontent.com/mesamirh/MovieBox-Tui/main/install.sh | bash
```

या Homebrew से (macOS):
```bash
brew tap mesamirh/moviebox-tui https://github.com/mesamirh/MovieBox-Tui
brew install moviebox-tui
```
> [!NOTE]
> यदि Homebrew पहली बार इंस्टॉल करते समय वेरिफिकेशन मांगे, तो `brew trust mesamirh/moviebox-tui` चलाएं।

### Windows

Scoop से:
```powershell
scoop bucket add moviebox https://github.com/mesamirh/MovieBox-Tui
scoop install moviebox-tui
```

या ऑटोमेटेड PowerShell स्क्रिप्ट से:
```powershell
irm https://raw.githubusercontent.com/mesamirh/MovieBox-Tui/main/install.ps1 | iex
```

### Android (Termux)

Termux खोलें और चलाएं:
```bash
pkg update && pkg install -y curl tar termux-tools termux-am
curl -fsSL https://raw.githubusercontent.com/mesamirh/MovieBox-Tui/main/install.sh | bash
termux-setup-storage
```
> [!IMPORTANT]
> एंड्रॉइड पर वीडियो प्लेबैक आपके इंस्टॉल किए गए बाहरी मीडिया प्लेयर (जैसे VLC, Just Player, या MX Player) के माध्यम से शुरू होता है।

<details>
<summary><b>Cargo और सोर्स कोड से बिल्ड करें</b></summary>

crates.io से:
```bash
cargo install moviebox-tui --locked
```

सोर्स कोड से:
```bash
git clone https://github.com/mesamirh/MovieBox-Tui.git
cd MovieBox-Tui
cargo build --release --locked
```

</details>

<details>
<summary><b>रिलीज की सत्यता जांचें (Verification)</b></summary>

```bash
sha256sum -c SHA256SUMS --ignore-missing
gh attestation verify <archive-file> -R mesamirh/MovieBox-Tui
```

</details>

<details>
<summary><b>अनइंस्टॉलेशन (Uninstallation)</b></summary>

#### ऑटोमेटेड इंस्टॉलर (macOS, Linux, Windows, Android)

बस अपना मूल इंस्टॉलेशन कमांड (`curl ... | bash` या `irm ... | iex`) फिर से चलाएं। जब MovieBox-TUI पहले से इंस्टॉल होता है, तो इंस्टॉलर इसे अपने आप पहचान लेता है और एक मेनू प्रदर्शित करता है:

```text
MovieBox-TUI is already installed.
What would you like to do?
  1) Reinstall / Update to latest version
  2) Uninstall
  3) Cancel
```

MovieBox-TUI को पूरी तरह हटाने के लिए `2` दर्ज करें।

#### पैकेज मैनेजर्स

```bash
brew uninstall moviebox-tui     # Homebrew (macOS)
scoop uninstall moviebox-tui    # Scoop (Windows)
cargo uninstall moviebox-tui    # Cargo
```

</details>

## शुरुआत कैसे करें

```bash
moviebox-tui
```

- सर्च करने के लिए कोई भी टाइटल टाइप करें, प्ले करने के लिए `Enter` दबाएं।
- शॉर्टकट्स के लिए TUI में `?` दबाएं, सेटिंग्स के लिए `/settings` टाइप करें।

## डॉक्यूमेंटेशन

विस्तृत गाइड और आर्किटेक्चर संदर्भ [**mesamirh.github.io/MovieBox-Tui**](https://mesamirh.github.io/MovieBox-Tui/) पर या [`docs/`](docs/) डायरेक्टरी में उपलब्ध हैं:

| गाइड | विवरण |
| :--- | :--- |
| [कीबोर्ड और कंट्रोल्स](docs/controls.md) | कीबाइंडिंग्स, vim नेविगेशन, सर्च और शॉर्टकट्स |
| [कॉन्फ़िगरेशन](docs/config.md) | सेटिंग्स, थीम कस्टमाइज़ेशन और कॉन्फ़िगरेशन विकल्प |
| [कंटेंट प्रोवाइडर्स](docs/providers.md) | नेटिव स्क्रेपर्स (MovieBox, Dramachi, BDIX) और स्ट्रीम एक्सट्रैक्टर्स |
| [Stremio ऐड-ऑन्स](docs/addons-mode.md) | कम्युनिटी ऐड-ऑन इंस्टॉलेशन, मैनिफेस्ट URL और स्ट्रीम रेजोल्यूशन |
| [हार्डवेयर प्लेयर्स](docs/players.md) | प्लेयर डिटेक्शन, लॉन्च ऑप्शंस और हार्डवेयर एक्सेलेरेशन |
| [लाइव टीवी और IPTV](docs/tv-mode.md) | M3U प्लेलिस्ट इम्पोर्ट, चैनल सर्च और लाइव स्ट्रीमिंग |
| [बैच डाउनलोड](docs/downloads.md) | पॉज और रिज्यूम के साथ मल्टी-सेगमेंट HTTP रेंज डाउनलोड |

## योगदान

प्रोजेक्ट में योगदान का स्वागत है। पुल रिक्वेस्ट सबमिट करने से पहले [CONTRIBUTING.md](CONTRIBUTING.md) की समीक्षा करें।

बग रिपोर्ट करने या नए फीचर का अनुरोध करने के लिए [GitHub Issues](https://github.com/mesamirh/MovieBox-Tui/issues) का उपयोग करें।

<details>
<summary><b>वैकल्पिक सहयोग (Optional Support)</b></summary>
<div id="optional-support" tabindex="-1"></div>

यदि आप सीधे निरंतर विकास का समर्थन करना चाहते हैं:

| नेटवर्क / क्रिप्टोकरेंसी | पता (Address) |
| :--- | :--- |
| **USDT (TRC20)** | `TL4yW73qmbKZpBWwbEFgjBpwVkPDFTkJgV` |
| **Bitcoin (BTC)** | `3MEAtqtRWrQBhnaMi3Zuf5nt2efNUS2LUQ` |
| **Ethereum / EVM** | `0x7ea20d5fa29d87f33195f5a3b211ff94038d794c` |
| **Solana (SOL)** | `6ctm5WFv73MNywoCKAz3xK72yizSspHa72rFNygooU6` |

</details>

## गोपनीयता (Privacy)

MovieBox-TUI में शून्य टेलीमेट्री, एनालिटिक्स या यूजर ट्रैकिंग है। सभी सर्च हिस्ट्री, बुकमार्क्स और कॉन्फ़िगरेशन फाइलें पूरी तरह से आपके स्थानीय फाइलसिस्टम पर ही रहती हैं।

## लाइसेंस

यह सॉफ्टवेयर [MIT](LICENSE-MIT) या [Apache-2.0](LICENSE-APACHE) के तहत लाइसेंस प्राप्त है।

## अस्वीकरण (Disclaimer)

यह प्रोजेक्ट किसी भी मीडिया को होस्ट या स्टोर नहीं करता है। यह सार्वजनिक रूप से उपलब्ध स्ट्रीम्स चलाने के लिए केवल एक स्वतंत्र क्लाइंट है। उपयोगकर्ता अपने देश के कानूनों का पालन करने के लिए स्वयं जिम्मेदार हैं।
