<div align="center">

# MovieBox-TUI

**লোকাল মিডিয়া প্লেয়ার দিয়ে মুভি, টিভি শো এবং লাইভ টিভি খোঁজা, ডাউনলোড ও স্ট্রিম করার টার্মিনাল ইন্টারফেস।**

[ English ](README.md) • [ বাংলা ](README_BN.md) • [ हिन्दी ](README_HI.md) • [ Español ](README_ES.md)

[![CI](https://img.shields.io/github/actions/workflow/status/mesamirh/MovieBox-Tui/ci.yml?branch=main&label=CI&logo=github&style=flat)](https://github.com/mesamirh/MovieBox-Tui/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/moviebox-tui.svg?logo=rust&style=flat)](https://crates.io/crates/moviebox-tui)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg?style=flat)](#license)
[![Telegram](https://img.shields.io/badge/Telegram-Channel-2CA5E0?style=flat&logo=telegram&logoColor=white)](https://t.me/getfromme)
</div>

[moviebox-tui-walkthrough.webm](https://github.com/user-attachments/assets/7554a7e5-6ff5-49ec-9d87-f821ea99950e)

## স্পন্সর (Sponsors)

এখানে আপনার ব্র্যান্ড বা প্রোডাক্ট দেখাতে চাইলে [et6253013@gmail.com](mailto:et6253013@gmail.com)-এ যোগাযোগ করুন।

## মূল সুবিধাসমূহ

- **স্ট্রিমিং:** একাধিক নেটিভ প্রোভাইডার এবং কমিউনিটি Stremio অ্যাড-অন থেকে মুভি, টিভি সিরিজ, অ্যানিমে ও এশিয়ান ড্রামা স্ট্রিম করুন।
- **লাইভ টিভি:** চ্যানেল ক্যাটাগরি, EPG সাপোর্ট এবং সার্চ সহ M3U প্লেলিস্ট ইমপোর্ট।
- **রেজোলিউশন পিকার:** প্লেব্যাকের আগে সরাসরি স্ট্রিম কোয়ালিটি (`4K`, `1080p`, `720p`, `480p`, `Auto`) নির্বাচন।
- **হার্ডওয়্যার প্লেয়ার:** কাস্টম অথেনটিকেশন হেডার ও কুকি ফরওয়ার্ডিং সহ সরাসরি `mpv`, `VLC`, বা `IINA`-তে প্লেব্যাক।
- **ব্যাচ ডাউনলোডার:** HTTP রেঞ্জ পজ ও রিজুম সাপোর্ট সহ একক পর্ব এবং পুরো সিজনের মাল্টি-সেগমেন্ট ডাউনলোড।
- **সাবটাইটেল পিকার:** প্লেব্যাক বা ডাউনলোডের আগে ইন্টারঅ্যাক্টিভ পিকারের মাধ্যমে একাধিক ভাষার সাবটাইটেল ট্র্যাক নির্বাচন।
- **টার্মিনাল UI:** Vim নেভিগেশন, মাউস ইন্টারঅ্যাকশন, স্ল্যাশ কমান্ড প্যালেট (`/help`, `/settings`), এবং ৯টি বিল্ট-ইন থিম।
- **কভার আর্ট:** স্বয়ংক্রিয় টেক্সট ফলব্যাক সহ নেটিভ Kitty, Sixel এবং iTerm2 পোস্টার রেন্ডারিং।
- **রিজুম ও লাইব্রেরি:** কন্টিনিউ-ওয়াচিং টাইমস্ট্যাম্প, ওয়াচ হিস্ট্রি এবং ফেভারিটস সম্পূর্ণ লোকাল ডিস্কে সংরক্ষিত থাকে। জিরো টেলিমেট্রি।

## পূর্বশর্তসমূহ

- **মিডিয়া প্লেয়ার:** `mpv`, `VLC`, বা `IINA` (macOS) / যেকোনো এক্সটার্নাল ভিডিও প্লেয়ার (Android)।
- **পোস্টার:** Sixel, Kitty বা iTerm2 সাপোর্টযুক্ত টার্মিনাল (Ghostty, Kitty, WezTerm, iTerm2, foot, Windows Terminal v1.22+)।
- **DASH ডাউনলোড:** `yt-dlp` এবং `ffmpeg` (শুধুমাত্র MovieBox DASH ডাউনলোডের জন্য প্রয়োজনীয়)।

## ইনস্টলেশন

### macOS ও Linux

আপনার সিস্টেমে [Homebrew](https://brew.sh/) থাকলে (macOS):
```bash
brew tap mesamirh/moviebox-tui https://github.com/mesamirh/MovieBox-Tui
brew install moviebox-tui
```

> **নোট:** প্রথমবার ইনস্টলের সময় Homebrew যদি ট্যাপ ভেরিফিকেশন চায়, তবে `brew trust mesamirh/moviebox-tui` রান করুন।

সরাসরি টার্মিনাল দিয়ে ইনস্টল (macOS ও Linux, কোনো প্যাকেজ ম্যানেজার লাগবে না):
```bash
curl -fsSL https://raw.githubusercontent.com/mesamirh/MovieBox-Tui/main/install.sh | bash
```

### Windows

আপনার সিস্টেমে [Scoop](https://scoop.sh/) থাকলে (সুপারিশকৃত):
```powershell
scoop bucket add moviebox https://github.com/mesamirh/MovieBox-Tui
scoop install moviebox-tui
```

PowerShell স্ক্রিপ্ট দিয়ে সরাসরি ইনস্টল (কোনো প্যাকেজ ম্যানেজার লাগবে না):
```powershell
irm https://raw.githubusercontent.com/mesamirh/MovieBox-Tui/main/install.ps1 | iex
```

> **SmartScreen প্রম্পট:** Windows যদি *"Windows protected your PC"* দেখায়, তবে **More info** → **Run anyway** এ ক্লিক করুন।

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
<summary><b>আনইনস্টল (Uninstall)</b></summary>

ইনস্টল কমান্ডটি (`curl ... | bash` অথবা `irm ... | iex`) পুনরায় রান করে `2) Uninstall` নির্বাচন করুন।

অথবা প্যাকেজ ম্যানেজারের মাধ্যমে:
```bash
brew uninstall moviebox-tui     # Homebrew
scoop uninstall moviebox-tui    # Scoop
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
| [কনটেন্ট প্রোভাইডার](docs/providers.md) | নেটিভ স্ক্র্যাপার (MovieBox, 4KHDHub, Dramachi, BDIX) |
| [Stremio অ্যাড-অন](docs/addons-mode.md) | কমিউনিটি অ্যাড-অন ইনস্টলেশন, ম্যানিফেস্ট URL এবং স্ট্রিম রেজোলিউশন |
| [হার্ডওয়্যার প্লেয়ার](docs/players.md) | প্লেয়ার ডিটেকশন, লঞ্চ অপশন এবং হার্ডওয়্যার অ্যাক্সিলারেশন |
| [লাইভ টিভি ও IPTV](docs/tv-mode.md) | M3U প্লেলিস্ট ইমপোর্ট, চ্যানেল সার্চ এবং লাইভ স্ট্রিমিং |
| [ব্যাচ ডাউনলোড](docs/downloads.md) | পজ ও রিজুম সাপোর্ট সহ মাল্টি-সেগমেন্ট HTTP রেঞ্জ ডাউনলোড |

## কন্ট্রিবিউশন

প্রজেক্টে যেকোনো ধরনের অবদান সাদরে আমন্ত্রিত। পুল রিকোয়েস্ট পাঠানোর আগে [CONTRIBUTING.md](CONTRIBUTING.md) গাইডলাইনটি দেখে নিন।

কোনো বাগ রিপোর্ট করতে বা নতুন ফিচারের অনুরোধ জানাতে [GitHub Issues](https://github.com/mesamirh/MovieBox-Tui/issues) ব্যবহার করুন।

## গোপনীয়তা (Privacy)

MovieBox-TUI সম্পূর্ণ টেলিমেট্রি ও ট্র্যাকিং মুক্ত। সমস্ত সার্চ হিস্ট্রি, বুকমার্ক এবং কনফিগারেশন ফাইল কেবল আপনার লোকাল ফাইলসিস্টেমেই সুরক্ষিত থাকে।

## লাইসেন্স

এই প্রজেক্টটি [MIT](LICENSE-MIT) অথবা [Apache-2.0](LICENSE-APACHE) লাইসেন্সের অধীনে প্রকাশিত।

## দাবিত্যাগ (Disclaimer)

এই প্রজেক্টটি নিজে কোনো মিডিয়া বা ভিডিও ফাইল হোস্ট বা সংরক্ষণ করে না। এটি ইন্টারনেটে উন্মুক্ত থাকা ভিডিও স্ট্রিমগুলো চালানোর একটি স্বাধীন টার্মিনাল ক্লায়েন্ট মাত্র। ব্যবহারকারীরা তাদের নিজ দেশের নিয়মকানুন মেনে চলার জন্য নিজেই দায়িত্বশীল থাকবেন।
