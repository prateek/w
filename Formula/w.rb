class W < Formula
  desc "Experimental multi-repo wrapper for Worktrunk"
  homepage "https://github.com/prateek/w"

  # HEAD-only for now. A stable release URL will be added once the repo publishes tagged releases.
  head "https://github.com/prateek/w.git", branch: "main"

  depends_on "rust" => :build

  def install
    system "cargo", "install", "--locked", *std_cargo_args(path: "crates/w")
  end

  test do
    assert_match "zsh", shell_output("#{bin}/w shell init zsh")
  end
end
