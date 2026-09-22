<div align="center">

# MovieBox-TUI

**अपने पसंदीदा मीडिया प्लेयर से फिल्में, वेब सीरीज और लाइव टीवी सर्च करने, डाउनलोड और स्ट्रीम करने का आसान टर्मिनल ऐप।**

[ English ](README.md) • [ বাংলা ](README_BN.md) • [ हिन्दी ](README_HI.md) • [ Español ](README_ES.md)

[![Telegram](https://telegram-badge.vercel.app/api/telegram-badge?channelId=@getfromme&style=flat&logo=true)](https://t.me/getfromme)
[![Donate](https://img.shields.io/badge/Donate-Crypto-F7931A?style=flat&logo=bitcoin&logoColor=white)](#optional-support)
</div>

[moviebox-tui-walkthrough.webm](https://github.com/user-attachments/assets/7554a7e5-6ff5-49ec-9d87-f821ea99950e)

## खास फीचर्स

- **मूवी और वेब सीरीज स्ट्रीमिंग**: MovieBox, 4KHDHub, BDIX सर्वर और Stremio ऐड-ऑन्स से सीधे फिल्में, सीरीज और एनीमे देखें।
- **लाइव टीवी (IPTV)**: कोई भी M3U प्लेलिस्ट लिंक जोड़कर आसानी से टीवी चैनल्स सर्च करें और लाइव देखें।
- **स्मूथ वीडियो प्लेबैक**: बिना किसी भारी ब्राउज़र के सीधे आपके पसंदीदा प्लेयर (`mpv`, `IINA`, `VLC` या एंड्रॉइड प्लेयर) में बिना लैग के चलेगा।
- **ऑटो सबटाइटल**: आपकी पसंदीदा भाषा में सबटाइटल खुद ढूंढकर सीधे प्लेयर में लोड कर देता है।
- **फास्ट डाउनलोड**: पूरा सीजन या कोई एक एपिसोड एक क्लिक में डाउनलोड करें, पॉज़ और रीज्यूम के पूरे सपोर्ट के साथ।
- **क्वालिटी कंट्रोल**: एक बार अपनी पसंदीदा रेज़ोल्यूशन चुन लें, वही हर स्ट्रीम और डाउनलोड पर लागू होगी, धीमे इंटरनेट पर डेटा बचाने के लिए बढ़िया।
- **कवर आर्ट और पोस्टर**: टर्मिनल विंडो के अंदर ही फिल्मों और सीरीज के रंगीन पोस्टर्स देख सकते हैं।
- **हिस्ट्री और बुकमार्क**: पसंदीदा शो को बुकमार्क करें और जहां देखना छोड़ा था, वहीं से दोबारा शुरू करें।
- **शानदार थीम्स**: आपके टर्मिनल लुक से मैच करने के लिए 6 इन-बिल्ट कलर थीम्स।
- **हर डिवाइस पर काम करे**: Mac, Linux, Windows और Android (Termux) सभी पर एकदम परफेक्ट काम करता है।

## क्या-क्या चाहिए

वीडियो देखने के लिए बस कोई एक प्लेयर होना चाहिए:

- **mpv** (Mac, Linux और Windows के लिए बेस्ट)
- **IINA** (Mac के लिए)
- **VLC** (सभी प्लेटफॉर्म्स के लिए)
- **फोन पर कोई भी वीडियो प्लेयर** Termux के जरिए (VLC, Just Player, MX Player)

*पोस्टर देखने के लिए:* टर्मिनल में पोस्टर्स देखने के लिए मॉडर्न टर्मिनल (Ghostty, Kitty, WezTerm या iTerm2) चाहिए। नॉर्मल टर्मिनल में टेक्स्ट लेआउट दिखेगा।

*MovieBox से डाउनलोड के लिए:* सिर्फ MovieBox से डाउनलोड करने के लिए `yt-dlp` और `ffmpeg` की जरूरत होती है। बाकी सभी प्रोवाइडर्स से ऐप खुद सीधे डाउनलोड कर लेता है।

## इंस्टॉल करने का तरीका

### Mac और Linux

ऑटोमैटिक इंस्टॉलेशन:
```bash
curl -fsSL https://raw.githubusercontent.com/mesamirh/MovieBox-Tui/main/install.sh | bash
```

Homebrew (Mac):
```bash
brew tap mesamirh/moviebox-tui https://github.com/mesamirh/MovieBox-Tui
brew install moviebox-tui
```
*नोट:* अगर Homebrew वेरिफिकेशन मांगे, तो `brew trust mesamirh/moviebox-tui` रन करें।

### Windows

PowerShell में रन करें:
```powershell
irm https://raw.githubusercontent.com/mesamirh/MovieBox-Tui/main/install.ps1 | iex
```

### Android (Termux)

1. जरूरी टूल्स इंस्टॉल करके रन करें:
```bash
pkg update && pkg install -y curl tar termux-tools termux-am
curl -fsSL https://raw.githubusercontent.com/mesamirh/MovieBox-Tui/main/install.sh | bash
```

2. स्टोरेज परमिशन दें (प्लेयर में वीडियो और सबटाइटल चलने के लिए):
```bash
termux-setup-storage
```

<details>
<summary><b>Cargo या सोर्स कोड से बिल्ड करें</b></summary>

crates.io से इंस्टॉल:
```bash
cargo install moviebox-tui --locked
```

सोर्स कोड से कंपाइल:
```bash
git clone https://github.com/mesamirh/MovieBox-Tui.git
cd MovieBox-Tui
cargo build --release --locked
```

</details>

<details>
<summary><b>रिलीज की सत्यता जांचें (Checksum)</b></summary>

```bash
sha256sum -c SHA256SUMS --ignore-missing
gh attestation verify <archive-file> -R mesamirh/MovieBox-Tui
```

</details>

## शुरू कैसे करें

```bash
moviebox-tui
```

- किसी भी मूवी या शो का नाम लिखकर सर्च करें, प्ले करने के लिए `Enter` दबाएं।
- शॉर्टकट्स देखने के लिए ऐप में कभी भी `?` दबाएं, सेटिंग्स के लिए `/settings` टाइप करें।

## डॉक्यूमेंटेशन

विस्तृत गाइड और आर्किटेक्चर समझने के लिए [**mesamirh.github.io/MovieBox-Tui**](https://mesamirh.github.io/MovieBox-Tui/) पर जाएं या प्रोजेक्ट का [`docs/`](docs/) फोल्डर देखें।

## योगदान (Contribution)

प्रोजेक्ट में आपके योगदान का स्वागत है। पुल रिक्वेस्ट भेजने से पहले [CONTRIBUTING.md](CONTRIBUTING.md) जरूर देखें।

कोई बग मिले या नए फीचर का आइडिया हो, तो बेझिझक [GitHub Issues](https://github.com/mesamirh/MovieBox-Tui/issues) पर बताएं।

<details>
<summary><b>डेवलपमेंट में सपोर्ट करें</b></summary>
<div id="optional-support" tabindex="-1"></div>

अगर आप प्रोजेक्ट डेवलपमेंट में सीधा सहयोग करना चाहते हैं:

| Network / Asset | Address |
| :--- | :--- |
| **USDT (TRC20)** | `TL4yW73qmbKZpBWwbEFgjBpwVkPDFTkJgV` |
| **Bitcoin (BTC)** | `3MEAtqtRWrQBhnaMi3Zuf5nt2efNUS2LUQ` |
| **Ethereum / EVM** | `0x7ea20d5fa29d87f33195f5a3b211ff94038d794c` |
| **Solana (SOL)** | `6ctm5WFv73MNywoCKAz3xK72yizSspHa72rFNygooU6` |

</details>

## प्राइवेसी और सुरक्षा

MovieBox-TUI कोई भी यूजर डेटा, टेलीमेट्री या एनालिटिक्स ट्रैक नहीं करता है। आपका सर्च इतिहास, बुकमार्क और सेटिंग्स सिर्फ और सिर्फ आपके अपने डिवाइस पर सुरक्षित रहते हैं।

## लाइसेंस

[MIT](LICENSE-MIT) या [Apache-2.0](LICENSE-APACHE) लाइसेंस के तहत जारी।

## डिस्क्लेमर

यह प्रोजेक्ट खुद कोई भी मीडिया या वीडियो फाइल स्टोर या होस्ट नहीं करता है। यह इंटरनेट पर पब्लिकली उपलब्ध वीडियो स्ट्रीम्स चलाने के लिए केवल एक ओपन-सोर्स टर्मिनल ऐप है।
