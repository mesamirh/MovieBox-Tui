class MovieboxTui < Formula
  VERSION = "0.1.10"
  MACOS_SHA256 = "864af514829549c4cca74218e9b99fd7ef63b1a4c8d023a7ac0779d8e024d03a"
  LINUX_X64_SHA256 = "8dfa992f6ef699ff1673ff1c371d95868ee7895b5c41ad0565328f12855ea877"
  LINUX_ARM64_SHA256 = "5ee85f3a6115adaf7940d20ac300b27360dcc41e889b4fa4edd98d056e5739c4"

  desc "Stream movies, shows, anime, and live TV from your terminal"
  homepage "https://github.com/mesamirh/MovieBox-Tui"
  version VERSION
  license "MIT"

  on_macos do
    url "https://github.com/mesamirh/MovieBox-Tui/releases/download/v#{VERSION}/MovieBox_macOS_Universal.tar.gz"
    sha256 MACOS_SHA256
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/mesamirh/MovieBox-Tui/releases/download/v#{VERSION}/MovieBox_Linux_arm64.tar.gz"
      sha256 LINUX_ARM64_SHA256
    else
      url "https://github.com/mesamirh/MovieBox-Tui/releases/download/v#{VERSION}/MovieBox_Linux_x64.tar.gz"
      sha256 LINUX_X64_SHA256
    end
  end

  def install
    bin.install "moviebox-tui"
  end

  test do
    system "#{bin}/moviebox-tui", "--version"
  end
end
