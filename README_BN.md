<div align="center">

# MovieBox-TUI

**লোকাল মিডিয়া প্লেয়ার দিয়ে মুভি, টিভি শো এবং লাইভ টিভি খোঁজা, ডাউনলোড ও স্ট্রিম করার টার্মিনাল ইন্টারফেস।**

[ English ](README.md) • [ বাংলা ](README_BN.md) • [ हिन्दी ](README_HI.md) • [ Español ](README_ES.md)

[![CI](https://img.shields.io/github/actions/workflow/status/mesamirh/MovieBox-Tui/ci.yml?branch=main&label=CI&logo=github&style=flat)](https://github.com/mesamirh/MovieBox-Tui/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/moviebox-tui.svg?logo=rust&style=flat)](https://crates.io/crates/moviebox-tui)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg?style=flat)](#license)
[![Telegram](https://img.shields.io/badge/Telegram-Channel-2CA5E0?style=flat&logo=telegram&logoColor=white)](https://t.me/getfromme)
[![Support](https://img.shields.io/badge/Support-Crypto-F7931A?style=flat&logo=bitcoin&logoColor=white)](#optional-support)
</div>

[moviebox-tui-walkthrough.webm](https://github.com/user-attachments/assets/7554a7e5-6ff5-49ec-9d87-f821ea99950e)

## মূল সুবিধাসমূহ

- **অন-ডিমান্ড স্ট্রিমিং**: একাধিক নেটিভ প্রোভাইডার এবং কমিউনিটি Stremio অ্যাড-অন থেকে মুভি, টিভি সিরিজ, অ্যানিমে ও এশিয়ান ড্রামা সরাসরি স্ট্রিম করুন।
- **লাইভ টিভি ও IPTV**: কাস্টম M3U প্লেলিস্ট URL ইমপোর্ট করে সহজে টিভি চ্যানেল সার্চ করুন, ক্যাটাগরি ব্রাউজ করুন এবং লাইভ টিভি উপভোগ করুন।
- **কোয়ালিটি ও রেজোলিউশন নির্বাচন**: ডিটেইলস স্ক্রিন থেকে সরাসরি নিজের পছন্দের স্ট্রিম রেজোলিউশন (`1080p`, `720p`, `480p`) বেছে নিন।
- **হার্ডওয়্যার অ্যাক্সিলারেটেড প্লেব্যাক**: স্বয়ংক্রিয় অথেনটিকেশন ও কুকি ফরওয়ার্ডিং সহ সরাসরি আপনার পছন্দের লোকাল মিডিয়া প্লেয়ারে ভিডিও চলবে।
- **মাল্টি-সেগমেন্ট ডাউনলোডার**: HTTP রেঞ্জ পজ ও রিজুম সাপোর্ট সহ একসাথে একাধিক সেগমেন্টে কোনো একটি পর্ব বা পুরো সিজন ডাউনলোড করুন।
- **স্বয়ংক্রিয় সাবটাইটেল**: আপনার পছন্দের ভাষার সাবটাইটেল নিজে থেকেই খুঁজে এনে স্বয়ংক্রিয়ভাবে প্লেয়ারে লোড করে।
- **ইন্টারঅ্যাক্টিভ টার্মিনাল UI**: vim নেভিগেশন ও কমান্ড অটো-সাজেশন সহ সম্পূর্ণ কীবোর্ড ও মাউস সাপোর্ট।
- **ভিজ্যুয়াল পোস্টার ও থিম**: ৯টি চমৎকার বিল্ট-ইন থিম এবং স্বয়ংক্রিয় লাইট/ডার্ক মোড সনাক্তকরণ সহ সরাসরি টার্মিনালে কালারফুল পোস্টার প্রদর্শিত হয়।
- **লাইব্রেরি ও প্রোগ্রেস ট্র্যাকিং**: পছন্দের শো স্টার মার্ক করুন, ওয়াচ হিস্ট্রি ট্র্যাক করুন এবং যেখান থেকে দেখা বন্ধ করেছিলেন সেখান থেকেই আবার শুরু করুন।
- **ক্রস-প্ল্যাটফর্ম ও প্রাইভেট**: কোনো প্রকার টেলিমেট্রি বা ডেটা সংগ্রহ ছাড়াই macOS, Linux, Windows এবং Android (Termux) এ নেটিভভাবে চলে।

## পূর্বশর্তসমূহ

### সাপোর্টেড মিডিয়া প্লেয়ার

আপনার সিস্টেমে নিচের যেকোনো একটি মিডিয়া প্লেয়ার ইনস্টল থাকা আবশ্যক:

- **ডেস্কটপ (macOS, Linux, Windows):** [mpv](https://mpv.io/), [VLC](https://www.videolan.org/), অথবা [IINA](https://iina.io/) *(macOS)*।
- **অ্যান্ড্রয়েড (Termux):** যেকোনো এক্সটার্নাল ভিডিও প্লেয়ার ([VLC](https://play.google.com/store/apps/details?id=org.videolan.vlc), Just Player, অথবা MX Player)।

### টার্মিনাল গ্রাফিক্স (পোস্টার)

পোস্টার প্রদর্শন আপনার টার্মিনাল পরিবেশের ওপর ভিত্তি করে স্বয়ংক্রিয়ভাবে মানিয়ে নেয়:

- **গ্রাফিক্স সাপোর্ট টার্মিনাল:** হাই-রেজোলিউশনের মুভি ও সিরিজ পোস্টার নেটিভভাবে দেখায়।
- **সাধারণ টার্মিনাল:** টেক্সট আকারে গোছানো প্লেসহোল্ডার স্বয়ংক্রিয়ভাবে দেখায়।

### ঐচ্ছিক ডিপেনডেন্সি

- **`yt-dlp` ও `ffmpeg`:** শুধুমাত্র MovieBox প্রোভাইডার থেকে DASH স্ট্রিম ডাউনলোডের জন্য প্রয়োজনীয়। বাকি সব প্রোভাইডার সরাসরি ইন-বিল্ট HTTP ইঞ্জিন দিয়ে ডাউনলোড হয়।

## ইনস্টলেশন

### macOS ও Linux

টার্মিনাল ওপেন করে রান করুন:
```bash
curl -fsSL https://raw.githubusercontent.com/mesamirh/MovieBox-Tui/main/install.sh | bash
```

অথবা Homebrew দিয়ে (macOS):
```bash
brew tap mesamirh/moviebox-tui https://github.com/mesamirh/MovieBox-Tui
brew install moviebox-tui
```
> [!NOTE]
> প্রথমবার ইনস্টলের সময় Homebrew যদি ট্যাপ ভেরিফিকেশন চায়, তবে `brew trust mesamirh/moviebox-tui` রান করুন।

### Windows

Scoop এর মাধ্যমে:
```powershell
scoop bucket add moviebox https://github.com/mesamirh/MovieBox-Tui
scoop install moviebox-tui
```

অথবা অটোমেটেড PowerShell স্ক্রিপ্ট:
```powershell
irm https://raw.githubusercontent.com/mesamirh/MovieBox-Tui/main/install.ps1 | iex
```

### Android (Termux)

Termux ওপেন করে রান করুন:
```bash
pkg update && pkg install -y curl tar termux-tools termux-am
curl -fsSL https://raw.githubusercontent.com/mesamirh/MovieBox-Tui/main/install.sh | bash
termux-setup-storage
```
> [!IMPORTANT]
> অ্যান্ড্রয়েডে ভিডিও প্লেব্যাক আপনার ইনস্টল করা এক্সটার্নাল প্লেয়ারের (যেমন VLC, Just Player, বা MX Player) মাধ্যমে চালু হয়।

<details>
<summary><b>Cargo ও সোর্স কোড থেকে বিল্ড</b></summary>

crates.io থেকে ইনস্টল:
```bash
cargo install moviebox-tui --locked
```

সোর্স কোড থেকে কম্পাইল:
```bash
git clone https://github.com/mesamirh/MovieBox-Tui.git
cd MovieBox-Tui
cargo build --release --locked
```

</details>

<details>
<summary><b>রিলিজের সত্যতা যাচাই (Verification)</b></summary>

```bash
sha256sum -c SHA256SUMS --ignore-missing
gh attestation verify <archive-file> -R mesamirh/MovieBox-Tui
```

</details>

<details>
<summary><b>আনইনস্টলেশন (Uninstallation)</b></summary>

#### অটোমেটেড ইনস্টলার (macOS, Linux, Windows, Android)

আপনার মূল ইনস্টলেশন কমান্ডটি (`curl ... | bash` অথবা `irm ... | iex`) পুনরায় রান করুন। MovieBox-TUI ইনস্টল করা থাকলে ইনস্টলার স্বয়ংক্রিয়ভাবে তা সনাক্ত করে একটি মেনু দেখাবে:

```text
MovieBox-TUI is already installed.
What would you like to do?
  1) Reinstall / Update to latest version
  2) Uninstall
  3) Cancel
```

MovieBox-TUI সম্পূর্ণ মুছে ফেলতে `2` চাপুন।

#### প্যাকেজ ম্যানেজার

```bash
brew uninstall moviebox-tui     # Homebrew (macOS)
scoop uninstall moviebox-tui    # Scoop (Windows)
cargo uninstall moviebox-tui    # Cargo
```

</details>

## কুইক স্টার্ট

```bash
moviebox-tui
```

- যেকোনো টাইটেল লিখে সার্চ করুন, প্লে করতে `Enter` চাপুন।
- শর্টকাট দেখতে অ্যাপের ভেতর `?` চাপুন, সেটিংসের জন্য `/settings` লিখুন।

## ডকুমেন্টেশন

বিস্তারিত গাইড ও আর্কিটেকচার সম্পর্কে জানতে ভিজিট করুন [**mesamirh.github.io/MovieBox-Tui**](https://mesamirh.github.io/MovieBox-Tui/) অথবা প্রজেক্টের [`docs/`](docs/) ডিরেক্টরি দেখুন:

| গাইড | বিবরণ |
| :--- | :--- |
| [কীবোর্ড ও কন্ট্রোলস](docs/controls.md) | কীবাইন্ডিং, vim নেভিগেশন, সার্চ এবং শর্টকাট |
| [কনফিগারেশন](docs/config.md) | সেটিংস, থিম কাস্টমাইজেশন এবং কনফিগারেশন অপশন |
| [কনটেন্ট প্রোভাইডার](docs/providers.md) | নেটিভ স্ক্র্যাপার (MovieBox, Dramachi, BDIX) এবং স্ট্রিম এক্সট্র্যাক্টর |
| [Stremio অ্যাড-অন](docs/addons-mode.md) | কমিউনিটি অ্যাড-অন ইনস্টলেশন, ম্যানিফেস্ট URL এবং স্ট্রিম রেজোলিউশন |
| [হার্ডওয়্যার প্লেয়ার](docs/players.md) | প্লেয়ার ডিটেকশন, লঞ্চ অপশন এবং হার্ডওয়্যার অ্যাক্সিলারেশন |
| [লাইভ টিভি ও IPTV](docs/tv-mode.md) | M3U প্লেলিস্ট ইমপোর্ট, চ্যানেল সার্চ এবং লাইভ স্ট্রিমিং |
| [ব্যাচ ডাউনলোড](docs/downloads.md) | পজ ও রিজুম সাপোর্ট সহ মাল্টি-সেগমেন্ট HTTP রেঞ্জ ডাউনলোড |

## কন্ট্রিবিউশন

প্রজেক্টে যেকোনো ধরনের অবদান সাদরে আমন্ত্রিত। পুল রিকোয়েস্ট পাঠানোর আগে [CONTRIBUTING.md](CONTRIBUTING.md) গাইডলাইনটি দেখে নিন।

কোনো বাগ রিপোর্ট করতে বা নতুন ফিচারের অনুরোধ জানাতে [GitHub Issues](https://github.com/mesamirh/MovieBox-Tui/issues) ব্যবহার করুন।

<details>
<summary><b>ঐচ্ছিক সহায়তা (Optional Support)</b></summary>
<div id="optional-support" tabindex="-1"></div>

প্রজেক্টের নিয়মিত উন্নয়নে সরাসরি সহায়তা করতে চাইলে:

| নেটওয়ার্ক / ক্রিপ্টোকারেন্সি | অ্যাড্রেস |
| :--- | :--- |
| **USDT (TRC20)** | `TL4yW73qmbKZpBWwbEFgjBpwVkPDFTkJgV` |
| **Bitcoin (BTC)** | `3MEAtqtRWrQBhnaMi3Zuf5nt2efNUS2LUQ` |
| **Ethereum / EVM** | `0x7ea20d5fa29d87f33195f5a3b211ff94038d794c` |
| **Solana (SOL)** | `6ctm5WFv73MNywoCKAz3xK72yizSspHa72rFNygooU6` |

</details>

## গোপনীয়তা (Privacy)

MovieBox-TUI সম্পূর্ণ টেলিমেট্রি ও ট্র্যাকিং মুক্ত। সমস্ত সার্চ হিস্ট্রি, বুকমার্ক এবং কনফিগারেশন ফাইল কেবল আপনার লোকাল ফাইলসিস্টেমেই সুরক্ষিত থাকে।

## লাইসেন্স

এই প্রজেক্টটি [MIT](LICENSE-MIT) অথবা [Apache-2.0](LICENSE-APACHE) লাইসেন্সের অধীনে প্রকাশিত।

## দাবিত্যাগ (Disclaimer)

এই প্রজেক্টটি নিজে কোনো মিডিয়া বা ভিডিও ফাইল হোস্ট বা সংরক্ষণ করে না। এটি ইন্টারনেটে উন্মুক্ত থাকা ভিডিও স্ট্রিমগুলো চালানোর একটি স্বাধীন টার্মিনাল ক্লায়েন্ট মাত্র। ব্যবহারকারীরা তাদের নিজ দেশের নিয়মকানুন মেনে চলার জন্য নিজেই দায়িত্বশীল থাকবেন।
