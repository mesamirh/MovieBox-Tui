class MovieboxTui < Formula
  VERSION = "0.1.29"
  MACOS_SHA256 = "d03d51a22e9329f3ac586d2edb1caf60e6edec39d465f3d55c85b9594176924a"
  LINUX_X64_SHA256 = "00d970f32b58fd8606a8c3f495425fe7960e3a96389c6b266f6920161a1587fe"
  LINUX_ARM64_SHA256 = "36ab1fb772ee9ab2f2daba8fef9b82325ec035b4dffec6a457049f1af05266d0"

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
