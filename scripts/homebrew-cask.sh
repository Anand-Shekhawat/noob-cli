#!/usr/bin/env bash
# Writes the Homebrew "cask" (the recipe for apps) for the Noob app. Usage:
#
#   scripts/homebrew-cask.sh v0.4.0 > ../homebrew-tap/Casks/noob-app.rb
set -euo pipefail

version_tag="${1:?Usage: scripts/homebrew-cask.sh v0.4.0}"
[[ "$version_tag" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]] || { echo "Not a version tag: $version_tag" >&2; exit 1; }
version="${version_tag#v}"
base="https://github.com/Anand-Shekhawat/noob-cli/releases/download/$version_tag"

# A fingerprint must be 64 hex letters; anything else means the download went wrong.
checksum() {
  local file="Noob-$version_tag-$1-apple-darwin.zip" sum
  sum="$(curl -fsSL "$base/$file.sha256" | cut -d ' ' -f 1)"
  [[ "$sum" =~ ^[0-9a-f]{64}$ ]] || { echo "Bad or missing checksum for $file" >&2; exit 1; }
  echo "$sum"
}
sum_arm="$(checksum aarch64)"
sum_intel="$(checksum x86_64)"

cat <<CASK
cask "noob-app" do
  arch arm: "aarch64", intel: "x86_64"

  version "$version"
  sha256 arm:   "$sum_arm",
         intel: "$sum_intel"

  url "https://github.com/Anand-Shekhawat/noob-cli/releases/download/v#{version}/Noob-v#{version}-#{arch}-apple-darwin.zip"
  name "Noob"
  desc "Beginner-friendly command line in its own window"
  homepage "https://github.com/Anand-Shekhawat/noob-cli"

  app "Noob.app"

  caveats <<~EOS
    Noob isn't signed by Apple yet. The first time you open it, macOS may say it
    can't be opened: go to System Settings, then Privacy & Security, and click
    "Open Anyway" next to Noob.
  EOS
end
CASK
