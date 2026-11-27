<div align="center">

# MovieBox-TUI

**स्थानीय मीडिया प्लेयर से फिल्में, टीवी शो और लाइव टीवी सर्च करने, डाउनलोड और स्ट्रीम करने का टर्मिनल इंटरफेस।**

[ English ](README.md) • [ বাংলা ](README_BN.md) • [ हिन्दी ](README_HI.md) • [ Español ](README_ES.md)

[![CI](https://img.shields.io/github/actions/workflow/status/mesamirh/MovieBox-Tui/ci.yml?branch=main&label=CI&logo=github&style=flat)](https://github.com/mesamirh/MovieBox-Tui/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/moviebox-tui.svg?logo=rust&style=flat)](https://crates.io/crates/moviebox-tui)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg?style=flat)](#license)
[![Telegram](https://img.shields.io/badge/Telegram-Channel-2CA5E0?style=flat&logo=telegram&logoColor=white)](https://t.me/getfromme)
</div>

[moviebox-tui-walkthrough.webm](https://github.com/user-attachments/assets/7554a7e5-6ff5-49ec-9d87-f821ea99950e)

## प्रायोजक (Sponsors)

यहाँ अपना ब्रांड या प्रोडक्ट दिखाने के लिए [et6253013@gmail.com](mailto:et6253013@gmail.com) पर संपर्क करें।

## खास फीचर्स

- **स्ट्रीमिंग:** कई नेटिव प्रोवाइडर्स और कम्युनिटी Stremio ऐड-ऑन्स से फिल्में, टीवी सीरीज, एनीमे और एशियन ड्रामा।
- **लाइव टीवी:** चैनल कैटेगरी, EPG सपोर्ट और सर्च के साथ M3U प्लेलिस्ट इम्पोर्ट।
- **रेजोल्यूशन पिकर:** प्लेबैक से पहले सीधे स्ट्रीम क्वालिटी (`4K`, `1080p`, `720p`, `480p`, `Auto`) का चयन।
- **हार्डवेयर प्लेयर्स:** कस्टम ऑथेंटिकेशन हेडर और कुकी फॉरवर्डिंग के साथ `mpv`, `VLC`, या `IINA` में सीधा लॉन्च।
- **बैच डाउनलोडर:** एपिसोड और पूरे सीजन के लिए HTTP रेंज पॉज और रिज्यूम के साथ मल्टी-सेगमेंट डाउनलोडर।
- **सबटाइटल पिकर:** प्लेबैक या डाउनलोड से पहले इंटरैक्टिव पिकर के माध्यम से बहु-भाषा सबटाइटल ट्रैक का चयन।
- **टर्मिनल UI:** Vim नेविगेशन, माउस इंटरैक्शन, स्लैश कमांड पैलेट (`/help`, `/settings`), और 9 इन-बिल्ट थीम्स।
- **कवर आर्ट:** ऑटोमैटिक टेक्स्ट फॉलबैक के साथ नेटिव Kitty, Sixel और iTerm2 पोस्टर रेंडरिंग।
- **रिज्यूम और लाइब्रेरी:** कंटिन्यू-वॉचिंग टाइमस्टैम्प, वॉच हिस्ट्री और फेवरेट्स स्थानीय डिस्क पर सुरक्षित। शून्य टेलीमेट्री।

## पूर्व-आवश्यकताएं

- **मीडिया प्लेयर:** `mpv`, `VLC`, या `IINA` (macOS) / कोई भी बाहरी वीडियो प्लेयर (Android)।
- **पोस्टर:** Sixel, Kitty, या iTerm2 सपोर्ट वाला टर्मिनल (Ghostty, Kitty, WezTerm, iTerm2, foot, Windows Terminal v1.22+)।
- **DASH डाउनलोड:** `yt-dlp` और `ffmpeg` (केवल MovieBox DASH डाउनलोड के लिए आवश्यक)।

## इंस्टॉलेशन

### macOS और Linux

यदि आपके पास macOS पर [Homebrew](https://brew.sh/) है:
```bash
brew tap mesamirh/moviebox-tui https://github.com/mesamirh/MovieBox-Tui
brew install moviebox-tui
```

> **नोट:** पहली बार इंस्टॉल करते समय यदि Homebrew वेरिफिकेशन मांगे, तो `brew trust mesamirh/moviebox-tui` चलाएं।

टर्मिनल से सीधे इंस्टॉल (macOS और Linux, किसी पैकेज मैनेजर की आवश्यकता नहीं):
```bash
curl -fsSL https://raw.githubusercontent.com/mesamirh/MovieBox-Tui/main/install.sh | bash
```

### Windows

यदि आपके पास [Scoop](https://scoop.sh/) है (अनुशंसित):
```powershell
scoop bucket add moviebox https://github.com/mesamirh/MovieBox-Tui
scoop install moviebox-tui
```

PowerShell स्क्रिप्ट से सीधे इंस्टॉल (किसी पैकेज मैनेजर की आवश्यकता नहीं):
```powershell
irm https://raw.githubusercontent.com/mesamirh/MovieBox-Tui/main/install.ps1 | iex
```

> **SmartScreen प्रॉम्ट:** यदि Windows *"Windows protected your PC"* दिखाता है, तो **More info** → **Run anyway** पर क्लिक करें।

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
<summary><b>अनइंस्टॉल (Uninstall)</b></summary>

इंस्टॉल कमांड (`curl ... | bash` या `irm ... | iex`) फिर से चलाएं और `2) Uninstall` चुनें।

या पैकेज मैनेजर से:
```bash
brew uninstall moviebox-tui     # Homebrew
scoop uninstall moviebox-tui    # Scoop
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
| [कंटेंट प्रोवाइडर्स](docs/providers.md) | नेटिव स्क्रेपर्स (MovieBox, 4KHDHub, Dramachi, BDIX) |
| [Stremio ऐड-ऑन्स](docs/addons-mode.md) | कम्युनिटी ऐड-ऑन इंस्टॉलेशन, मैनिफेस्ट URL और स्ट्रीम रेजोल्यूशन |
| [हार्डवेयर प्लेयर्स](docs/players.md) | प्लेयर डिटेक्शन, लॉन्च ऑप्शंस और हार्डवेयर एक्सेलेरेशन |
| [लाइव टीवी और IPTV](docs/tv-mode.md) | M3U प्लेलिस्ट इम्पोर्ट, चैनल सर्च और लाइव स्ट्रीमिंग |
| [बैच डाउनलोड](docs/downloads.md) | पॉज और रिज्यूम के साथ मल्टी-सेगमेंट HTTP रेंज डाउनलोड |

## योगदान

प्रोजेक्ट में योगदान का स्वागत है। पुल रिक्वेस्ट सबमिट करने से पहले [CONTRIBUTING.md](CONTRIBUTING.md) की समीक्षा करें।

बग रिपोर्ट करने या नए फीचर का अनुरोध करने के लिए [GitHub Issues](https://github.com/mesamirh/MovieBox-Tui/issues) का उपयोग करें।

## गोपनीयता (Privacy)

MovieBox-TUI में शून्य टेलीमेट्री, एनालिटिक्स या यूजर ट्रैकिंग है। सभी सर्च हिस्ट्री, बुकमार्क्स और कॉन्फ़िगरेशन फाइलें पूरी तरह से आपके स्थानीय फाइलसिस्टम पर ही रहती हैं।

## लाइसेंस

यह सॉफ्टवेयर [MIT](LICENSE-MIT) या [Apache-2.0](LICENSE-APACHE) के तहत लाइसेंस प्राप्त है।

## अस्वीकरण (Disclaimer)

यह प्रोजेक्ट किसी भी मीडिया को होस्ट या स्टोर नहीं करता है। यह सार्वजनिक रूप से उपलब्ध स्ट्रीम्स चलाने के लिए केवल एक स्वतंत्र क्लाइंट है। उपयोगकर्ता अपने देश के कानूनों का पालन करने के लिए स्वयं जिम्मेदार हैं।
