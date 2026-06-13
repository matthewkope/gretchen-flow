cask "gretchen-flow" do
  version "0.2.1"
  sha256 "be2669f63f8cdeb6b791397acd3c6245c2f29bf778321627f40030439fb63467"

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
