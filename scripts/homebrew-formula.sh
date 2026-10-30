#!/usr/bin/env bash
# Writes the Homebrew "formula" (recipe) for a Noob release, using the checksums
# the Release workflow uploaded. Usage:
#
#   scripts/homebrew-formula.sh v0.3.0 > ../homebrew-tap/Formula/noob.rb
set -euo pipefail

version_tag="${1:?Usage: scripts/homebrew-formula.sh v0.3.0}"
[[ "$version_tag" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]] || { echo "Not a version tag: $version_tag" >&2; exit 1; }
version="${version_tag#v}"
repo="Anand-Shekhawat/noob-cli"
base="https://github.com/$repo/releases/download/$version_tag"

# A fingerprint must be 64 hex letters; anything else means the download went wrong.
checksum() {
  local file="noob-$version_tag-$1.tar.gz" sum
  sum="$(curl -fsSL "$base/$file.sha256" | cut -d ' ' -f 1)"
  [[ "$sum" =~ ^[0-9a-f]{64}$ ]] || { echo "Bad or missing checksum for $file" >&2; exit 1; }
  echo "$sum"
}

# Work the checksums out first, so a failure stops before any formula is printed.
sum_arm="$(checksum aarch64-apple-darwin)"
sum_intel="$(checksum x86_64-apple-darwin)"
sum_linux="$(checksum x86_64-unknown-linux-gnu)"

cat <<FORMULA
class Noob < Formula
  desc "Beginner-friendly command line: type plain English, learn the real commands"
  homepage "https://github.com/$repo"
  version "$version"
  license "MIT"

  on_macos do
    on_arm do
      url "$base/noob-$version_tag-aarch64-apple-darwin.tar.gz"
      sha256 "$sum_arm"
    end
    on_intel do
      url "$base/noob-$version_tag-x86_64-apple-darwin.tar.gz"
      sha256 "$sum_intel"
    end
  end

  on_linux do
    url "$base/noob-$version_tag-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "$sum_linux"
  end

  def install
    bin.install "noob"
  end

  test do
    assert_match "Noob $version", shell_output("#{bin}/noob version")
  end
end
FORMULA
