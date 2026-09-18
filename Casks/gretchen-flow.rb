cask "gretchen-flow" do
  version "0.3.0"
  sha256 "387fae7e5041d386c66015245725e1f3ee3f5e82ffb152c5843e507f695fcabf"

  url "https://github.com/matthewkope/gretchen-flow/releases/download/v#{version}/Gretchen.Flow_#{version}_aarch64.dmg"
  name "Gretchen Flow"
  desc "Push-to-talk voice dictation with Parakeet, fully on-device"
  homepage "https://github.com/matthewkope/gretchen-flow"

  depends_on macos: :sonoma
  depends_on arch: :arm64

  app "Gretchen Flow.app"

  caveats <<~EOS
    Gretchen Flow ships without a speech model. On first launch a setup window
    guides you to download the recommended Parakeet v2 model (~464 MB).

    Grant Microphone, Accessibility, and Input Monitoring permissions when prompted
    (System Settings > Privacy & Security).

    This build is not yet notarized. If macOS refuses to open it, run:
      xattr -dr com.apple.quarantine "/Applications/Gretchen Flow.app"
  EOS
end
