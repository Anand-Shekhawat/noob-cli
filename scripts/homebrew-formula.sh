#!/usr/bin/env bash
# Writes the Homebrew "formula" (recipe) for a Noob release, using the checksums
# the Release workflow uploaded. Usage:
#
#   scripts/homebrew-formula.sh v0.3.0 > ../homebrew-tap/Formula/noob.rb
set -euo pipefail

version_tag="${1:?Usage: scripts/homebrew-formula.sh v0.3.0}"
version="${version_tag#v}"
repo="Anand-Shekhawat/noob-cli"
base="https://github.com/$repo/releases/download/$version_tag"

checksum() {
  local file="noob-$version_tag-$1.tar.gz"
  curl -fsSL "$base/$file.sha256" | cut -d ' ' -f 1
}

cat <<FORMULA
class Noob < Formula
  desc "Beginner-friendly command line: type plain English, learn the real commands"
  homepage "https://github.com/$repo"
  version "$version"
  license "MIT"

  on_macos do
    on_arm do
      url "$base/noob-$version_tag-aarch64-apple-darwin.tar.gz"
      sha256 "$(checksum aarch64-apple-darwin)"
    end
    on_intel do
      url "$base/noob-$version_tag-x86_64-apple-darwin.tar.gz"
      sha256 "$(checksum x86_64-apple-darwin)"
    end
  end

  on_linux do
    url "$base/noob-$version_tag-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "$(checksum x86_64-unknown-linux-gnu)"
  end

  def install
    bin.install "noob"
  end

  test do
    assert_match "Noob $version", shell_output("#{bin}/noob version")
  end
end
FORMULA
