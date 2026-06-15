cask "gretchen-flow" do
  version "0.2.2"
  sha256 "1bc4082ba41d96ee68d527d73f11ec41e2e1a1d18afcc54307e814efae764689"

  url "https://github.com/matthewkope/gretchen-flow/releases/download/v#{version}/Gretchen.Flow_#{version}_aarch64.dmg"
  name "Gretchen Flow"
  desc "Push-to-talk voice dictation powered by Whisper, fully on-device"
  homepage "https://github.com/matthewkope/gretchen-flow"

  depends_on macos: :monterey
  depends_on arch: :arm64

  app "Gretchen Flow.app"

  caveats <<~EOS
    Gretchen Flow ships without a speech model. On first launch a setup window
    guides you to download the recommended model (~547 MB).

    Grant Microphone and Accessibility permissions when prompted
    (System Settings > Privacy & Security).

    This build is not yet notarized. If macOS refuses to open it, run:
      xattr -dr com.apple.quarantine "/Applications/Gretchen Flow.app"
  EOS
end
