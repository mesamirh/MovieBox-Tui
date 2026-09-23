class MovieboxTui < Formula
  VERSION = "0.1.24"
  MACOS_SHA256 = "b8743e97a423c060a529ce1ab974ec2cec0cf9745096e3bf0dc93b4b6ae8c763"
  LINUX_X64_SHA256 = "fbb79f84c31f31d68d22adc157614d1af38b62b7343023c79eddb76360760f32"
  LINUX_ARM64_SHA256 = "431b98c89b6e184937c538c4121fc2a9f72d63fe8083e7ec524d54037eb8db90"

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
