# Formula template for ChrisWoo0443/homebrew-aliast (Formula/aliast.rb).
#
# Apply this at the NEXT release (v1.3.0+): those tarballs bundle
# aliast.plugin.zsh, so the separate plugin resource (and the loose
# aliast.plugin.zsh release asset) can be dropped. Do NOT apply for v1.2.0 --
# its tarballs contain only the binary.
#
# Release checklist:
#   1. Bump workspace version in Cargo.toml, tag vX.Y.Z, push the tag.
#   2. Wait for the Release workflow; copy the two sha256s from SHA256SUMS.
#   3. Fill VERSION and the sha256s below; push to the tap's Formula/aliast.rb.
#   4. Once no supported release needs it, remove the loose plugin asset step
#      from .github/workflows/release.yml (marked TODO).
class Aliast < Formula
  desc "Ghost-text autocompletion and natural-language commands for zsh"
  homepage "https://github.com/ChrisWoo0443/AliasT"
  version "REPLACE_VERSION"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/ChrisWoo0443/AliasT/releases/download/v#{version}/aliast-aarch64-apple-darwin.tar.gz"
      sha256 "REPLACE_ARM64_SHA256"
    elsif Hardware::CPU.intel?
      url "https://github.com/ChrisWoo0443/AliasT/releases/download/v#{version}/aliast-x86_64-apple-darwin.tar.gz"
      sha256 "REPLACE_X86_64_SHA256"
    end
  end

  def install
    bin.install "aliast"
    # The plugin ships inside the tarball since v1.3.0 -- one download,
    # one checksum, no separate resource.
    (share/"aliast").install "aliast.plugin.zsh"
  end

  def caveats
    <<~EOS
      To activate AliasT, add the following line to your ~/.zshrc:

        source #{HOMEBREW_PREFIX}/share/aliast/aliast.plugin.zsh

      Then restart your terminal or run:

        source ~/.zshrc

      The daemon (aliast) will start automatically the first time
      the plugin is loaded -- no manual startup required.
    EOS
  end

  test do
    assert_match "aliast", shell_output("#{bin}/aliast --version")
  end
end
