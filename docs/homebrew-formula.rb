# This is the initial template for segunmo/homebrew-meriadoc/Formula/meriadoc.rb
# The release workflow overwrites this file automatically on each release.
# SHA256 hashes are placeholders until the first release runs.

class Meriadoc < Formula
  desc "Discover, validate, and run tasks, jobs, and shells from project spec files"
  homepage "https://github.com/segunmo/meriadoc"
  version "0.1.0"
  license "MIT OR Apache-2.0"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/segunmo/meriadoc/releases/download/v0.1.0/meriadoc-v0.1.0-aarch64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER_ARM64"
    else
      url "https://github.com/segunmo/meriadoc/releases/download/v0.1.0/meriadoc-v0.1.0-x86_64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER_X86_64"
    end
  end

  def install
    bin.install "meriadoc"
    generate_completions_from_executable(bin/"meriadoc", "completions")
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/meriadoc --version")
  end
end
