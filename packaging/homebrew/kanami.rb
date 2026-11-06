# Homebrew formula template. After releasing a new version, update the version in url and the sha256,
# then commit it to the tap repository (such as KurumiRin/homebrew-tap). It builds from source, so
# Gatekeeper does not block it. See docs/releasing.md.
class Kanami < Formula
  desc "Tiny, fast, programmable keyboard remapper"
  homepage "https://github.com/KurumiRin/kanami"
  url "https://github.com/KurumiRin/kanami/archive/refs/tags/v0.1.0.tar.gz"
  sha256 "REPLACE_WITH_SHA256_OF_THE_TAG_TARBALL"
  license any_of: ["MIT", "Apache-2.0"]
  head "https://github.com/KurumiRin/kanami.git", branch: "main"

  depends_on "rust" => :build
  depends_on :macos

  def install
    system "cargo", "install", *std_cargo_args(path: "crates/kanami")
    pkgshare.install "config.example.toml", "plugins", "examples", "schema"
  end

  def caveats
    <<~EOS
      Example config and plugins are in:
        #{opt_pkgshare}

      Quick start:
        mkdir -p ~/.config/kanami
        cp #{opt_pkgshare}/config.example.toml ~/.config/kanami/config.toml
        kanami check && kanami test
        kanami service install

      On first run, grant permission in “System Settings → Privacy & Security → Accessibility”.
      The program's signature changes with every upgrade, so you must grant it again (remove the old entry first, then add it).
    EOS
  end

  test do
    (testpath/"config.toml").write <<~TOML
      [remap]
      grave = "escape"

      [[test]]
      input = "grave"
      expect = "escape"
    TOML
    system bin/"kanami", "check", "-c", testpath/"config.toml"
    system bin/"kanami", "test", "-c", testpath/"config.toml"
  end
end
