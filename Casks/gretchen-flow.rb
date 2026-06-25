cask "gretchen-flow" do
  version "0.2.5"
  sha256 "4cea6be04d21707fea4229a8080387485b90ce870d4411627153f8992c7d6578"

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
