class MovieboxTui < Formula
  desc "Stream movies, shows, anime, and live TV from your terminal"
  homepage "https://github.com/mesamirh/MovieBox-Tui"
  url "https://github.com/mesamirh/MovieBox-Tui/releases/download/v0.1.7/MovieBox_macOS_Universal.tar.gz"
  version "0.1.7"
  sha256 "a7ff0f876d7170531df514da366a24f339a229625e60941a1c18b3b2147b7efa"
  license "MIT"

  def install
    bin.install "moviebox-tui"
  end

  test do
    system "#{bin}/moviebox-tui", "--version"
  end
end
