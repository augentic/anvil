class Specify < Formula
  desc "Spec-driven development CLI for the Augentic change lifecycle"
  homepage "https://github.com/augentic/specify"
  url "https://github.com/augentic/specify/archive/refs/tags/v0.1.0.tar.gz"
  sha256 "PLACEHOLDER"
  license any_of: ["MIT", "Apache-2.0"]

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
    generate_completions_from_executable(bin/"specify", "completions")
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/specify --version")
  end
end
