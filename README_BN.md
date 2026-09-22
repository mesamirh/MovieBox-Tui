<div align="center">

# MovieBox-TUI

**আপনার কম্পিউটারের পছন্দের প্লেয়ার দিয়ে মুভি, টিভি শো আর লাইভ টিভি খোঁজা, ডাউনলোড ও স্ট্রিম করার টার্মিনাল অ্যাপ।**

[ English ](README.md) • [ বাংলা ](README_BN.md) • [ हिन्दी ](README_HI.md) • [ Español ](README_ES.md)

[![Telegram](https://telegram-badge.vercel.app/api/telegram-badge?channelId=@getfromme&style=flat&logo=true)](https://t.me/getfromme)
[![Donate](https://img.shields.io/badge/Donate-Crypto-F7931A?style=flat&logo=bitcoin&logoColor=white)](#optional-support)
</div>

[moviebox-tui-walkthrough.webm](https://github.com/user-attachments/assets/7554a7e5-6ff5-49ec-9d87-f821ea99950e)

## কী কী সুবিধা আছে

- **মুভি ও সিরিজ স্ট্রিমিং**: MovieBox, 4KHDHub, BDIX সার্ভার এবং Stremio অ্যাড-অন থেকে সহজে মুভি, সিরিজ ও অ্যানিমে দেখুন।
- **লাইভ টিভি (IPTV)**: যেকোনো M3U প্লেলিস্ট লিংক যোগ করে সরাসরি দেশি-বিদেশি টিভি চ্যানেল ব্রাউজ করুন ও দেখুন।
- **স্মুথ প্লেব্যাক**: কোনো বাড়তি ব্রাউজার ছাড়াই সরাসরি আপনার প্লেয়ারে (`mpv`, `IINA`, `VLC` বা ফোনে যেকোনো ভিডিও প্লেয়ার) স্মুথভাবে চলবে।
- **অটো সাবটাইটেল**: আপনার পছন্দের ভাষার সাবটাইটেল নিজে থেকেই খুঁজে প্লেয়ারে সেট করে দেবে।
- **সহজে ডাউনলোড**: পুরো সিজন বা যেকোনো একটা পর্ব এক ক্লিকেই ডাউনলোড করুন, সাথে পজ ও রিজুমের ফুল সাপোর্ট।
- **কোয়ালিটি কন্ট্রোল**: একবার পছন্দের রেজোলিউশন বেছে নিন, সেটাই প্রতিটি স্ট্রিম ও ডাউনলোডে ব্যবহার হবে, ধীর ইন্টারনেটে ডেটা বাঁচানোর জন্য দারুণ।
- **পোস্টার ও কভার আর্ট**: টার্মিনালের ভেতরেই মুভি আর সিরিজের কালারফুল পোস্টার দেখতে পারবেন।
- **হিস্ট্রি ও বুকমার্ক**: পছন্দের জিনিস সেভ করে রাখুন এবং যেখানে দেখা বন্ধ করেছিলেন, ঠিক সেখান থেকেই আবার দেখা শুরু করুন।
- **পছন্দের থিম**: আপনার টার্মিনালের সাথে মিলিয়ে নিতে ৬টি চমৎকার বিল্ট-ইন কালার থিম।
- **সব ডিভাইসে চলে**: Mac, Linux, Windows এবং Android (Termux) সব জায়গায় একদম পারফেক্টলি কাজ করে।

## যা যা লাগবে

ভিডিও দেখার জন্য যেকোনো একটি প্লেয়ার ইন্সটল থাকলেই চলবে:

- **mpv** (Mac, Linux ও Windows এর জন্য সবচেয়ে ভালো)
- **IINA** (Mac এর জন্য)
- **VLC** (সব ডিভাইসের জন্য)
- **ফোনে যেকোনো ভিডিও প্লেয়ার** Termux এর মাধ্যমে (VLC, Just Player, MX Player)

*পোস্টার ছবি দেখতে:* টার্মিনালে পোস্টার দেখতে চাইলে আধুনিক টার্মিনাল (Ghostty, Kitty, WezTerm বা iTerm2) লাগবে। সাধারণ টার্মিনালে পোস্টারের জায়গায় টেক্সট আকারে সুন্দরভাবে দেখাবে।

*MovieBox ডাউনলোডের জন্য:* শুধু MovieBox থেকে ডাউনলোড করতে `yt-dlp` আর `ffmpeg` প্রয়োজন হয়। অন্য সব প্রোভাইডার থেকে অ্যাপ নিজেই সরাসরি ডাউনলোড করতে পারে।

## ইন্সটল করার নিয়ম

### Mac ও Linux

অটোমেটিক ইন্সটল:
```bash
curl -fsSL https://raw.githubusercontent.com/mesamirh/MovieBox-Tui/main/install.sh | bash
```

Homebrew (Mac):
```bash
brew tap mesamirh/moviebox-tui https://github.com/mesamirh/MovieBox-Tui
brew install moviebox-tui
```
*নোট:* যদি Homebrew ভেরিফিকেশন চায়, তবে `brew trust mesamirh/moviebox-tui` রান করুন।

### Windows

PowerShell এ রান করুন:
```powershell
irm https://raw.githubusercontent.com/mesamirh/MovieBox-Tui/main/install.ps1 | iex
```

### Android (Termux)

১. প্রয়োজনীয় প্যাকেজগুলো ইন্সটল করে রান করুন:
```bash
pkg update && pkg install -y curl tar termux-tools termux-am
curl -fsSL https://raw.githubusercontent.com/mesamirh/MovieBox-Tui/main/install.sh | bash
```

২. স্টোরেজ পারমিশন দিন (প্লেয়ারে ভিডিও ও সাবটাইটেল চলার জন্য):
```bash
termux-setup-storage
```

<details>
<summary><b>Cargo দিয়ে বা সোর্স কোড থেকে বিল্ড</b></summary>

crates.io থেকে ইন্সটল:
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
<summary><b>রিলিজের সত্যতা যাচাই (Checksum)</b></summary>

```bash
sha256sum -c SHA256SUMS --ignore-missing
gh attestation verify <archive-file> -R mesamirh/MovieBox-Tui
```

</details>

## কীভাবে ব্যবহার করবেন

```bash
moviebox-tui
```

- যেকোনো মুভি বা সিরিজের নাম লিখে সার্চ করুন, প্লে করতে `Enter` চাপুন।
- শর্টকাট দেখতে অ্যাপে যেকোনো সময় `?` চাপুন, সেটিংসের জন্য `/settings` লিখুন।

## ডকুমেন্টেশন

বিস্তারিত গাইড ও আর্কিটেকচার সম্পর্কে জানতে ভিজিট করুন [**mesamirh.github.io/MovieBox-Tui**](https://mesamirh.github.io/MovieBox-Tui/) অথবা প্রজেক্টের [`docs/`](docs/) ফোল্ডারটি দেখুন।

## কন্ট্রিবিউশন

প্রজেক্টে যেকোনো ধরনের অবদান সাদরে আমন্ত্রিত। পুল রিকোয়েস্ট পাঠানোর আগে [CONTRIBUTING.md](CONTRIBUTING.md) গাইডলাইনটি দেখে নিন।

কোনো বাগ পেলে বা নতুন ফিচারের আইডিয়া থাকলে নির্দ্বিধায় [GitHub Issues](https://github.com/mesamirh/MovieBox-Tui/issues) এ জানান।

<details>
<summary><b>ডেভেলপমেন্টে সাপোর্ট করুন</b></summary>
<div id="optional-support" tabindex="-1"></div>

প্রজেক্টের নিয়মিত ডেভেলপমেন্টে সাপোর্ট করতে চাইলে:

| Network / Asset | Address |
| :--- | :--- |
| **USDT (TRC20)** | `TL4yW73qmbKZpBWwbEFgjBpwVkPDFTkJgV` |
| **Bitcoin (BTC)** | `3MEAtqtRWrQBhnaMi3Zuf5nt2efNUS2LUQ` |
| **Ethereum / EVM** | `0x7ea20d5fa29d87f33195f5a3b211ff94038d794c` |
| **Solana (SOL)** | `6ctm5WFv73MNywoCKAz3xK72yizSspHa72rFNygooU6` |

</details>

## প্রাইভেসি ও নিরাপত্তা

MovieBox-TUI কোনো প্রকার ইউজার ডেটা, টেলিমেট্রি বা অ্যানালিটিক্স ট্র্যাক করে না। আপনার সার্চ হিস্ট্রি, বুকমার্ক ও কনফিগারেশন ফাইল সম্পূর্ণভাবে আপনার নিজের ডিভাইসেই সুরক্ষিত থাকে।

## লাইসেন্স

[MIT](LICENSE-MIT) অথবা [Apache-2.0](LICENSE-APACHE) লাইসেন্সের অধীনে প্রকাশিত।

## ডিসক্লেইমার

এই প্রজেক্টটি নিজে কোনো মিডিয়া বা ভিডিও ফাইল হোস্ট বা সংরক্ষণ করে না। এটি ইন্টারনেটে উন্মুক্ত থাকা ভিডিও স্ট্রিমগুলো চালানোর একটি ওপেন-সোর্স টার্মিনাল ক্লায়েন্ট মাত্র। ব্যবহারকারীরা তাদের নিজ দেশের নিয়মকানুন মেনে চলার জন্য দায়িত্বশীল থাকবেন।
