class MovieboxTui < Formula
  VERSION = "0.1.30"
  MACOS_SHA256 = "2b7c2ba5c212b1fc0c554eaf5cddc7937bfed0fe74b2ed5321894b7a76bb5929"
  LINUX_X64_SHA256 = "7aa63669d7c68fbb1b001649edb674d6f543dc1152b57a16afb158ae2f4c6fdf"
  LINUX_ARM64_SHA256 = "67fca6aefe71d4f1fdeea0feaa70a4b1e319ca814005b04a5ddca1d0436ea7e5"

  desc "Stream movies, shows, anime, and live TV from your terminal"
  homepage "https://github.com/mesamirh/MovieBox-Tui"
  version VERSION
  license any_of: ["MIT", "Apache-2.0"]

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
