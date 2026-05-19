# Deep CLI — Release Spec

**Hedef:** `deepcanvas-cli` Rust binary'sinin paketlenmesi, dağıtımı, kurulum kanalları
**Kapsam:** Cross-compilation, GitHub Releases, install.sh, Homebrew tap, CI workflows
**Versiyon:** v1.0
**Bağımlılık:** `cli-implementation-spec.md` uygulanmış, compile geçen workspace mevcut.

> Bu spec **DevOps / Release engineering** kapsamındadır. Kodlama ajanı yerine repository + CI yapılandırması yapan kişi tarafından uygulanır.

---

## 1. Genel Akış

```
git tag v0.1.0 → push
        │
        ▼
GitHub Actions release workflow tetiklenir
        │
        ├─ 4 target için cross-compile
        ├─ Her target için tar.gz + checksums
        ├─ GitHub Release oluştur, asset'leri upload
        └─ Homebrew tap formula bump (otomatik PR)

Kullanıcı tarafı:
        ├─ curl cli.deepcanvas.studio/install.sh | sh
        ├─ brew install deepcanvas-studio/tap/deep
        └─ deep update (kendi kendine)
```

---

## 2. Target Matrix

| Target | Runner | Notlar |
|---|---|---|
| `aarch64-apple-darwin` | `macos-14` | M1+ Mac (native) |
| `x86_64-apple-darwin` | `macos-13` | Intel Mac (native) |
| `x86_64-unknown-linux-musl` | `ubuntu-22.04` | musl statik, glibc bağımsız |
| `aarch64-unknown-linux-musl` | `ubuntu-22.04` + cross | ARM Linux |

**Windows yok** — sonraki faz.

**Neden musl:** glibc-bound binary eski distrolarda crash; musl + rustls = tek dosya, her Linux'ta çalışır.

---

## 3. Asset Adlandırma

GitHub release asset'leri:

```
deep-v0.1.0-aarch64-apple-darwin.tar.gz
deep-v0.1.0-x86_64-apple-darwin.tar.gz
deep-v0.1.0-x86_64-unknown-linux-musl.tar.gz
deep-v0.1.0-aarch64-unknown-linux-musl.tar.gz
deep-v0.1.0-checksums.txt
```

**Arşiv içeriği:**
```
deep-v0.1.0-aarch64-apple-darwin/
├── deep                  ← binary, executable
├── LICENSE
└── README.md
```

**Checksums** (`deep-v0.1.0-checksums.txt`):
```
abc123...  deep-v0.1.0-aarch64-apple-darwin.tar.gz
def456...  deep-v0.1.0-x86_64-apple-darwin.tar.gz
...
```

`self_update` crate bu pattern'i otomatik tüketir.

---

## 4. GitHub Actions: `release.yml`

**Dosya:** `.github/workflows/release.yml`

```yaml
name: Release

on:
  push:
    tags:
      - 'v*'

permissions:
  contents: write

jobs:
  build:
    name: Build ${{ matrix.target }}
    runs-on: ${{ matrix.runner }}
    strategy:
      fail-fast: false
      matrix:
        include:
          - target: aarch64-apple-darwin
            runner: macos-14
            cross: false
          - target: x86_64-apple-darwin
            runner: macos-13
            cross: false
          - target: x86_64-unknown-linux-musl
            runner: ubuntu-22.04
            cross: false
          - target: aarch64-unknown-linux-musl
            runner: ubuntu-22.04
            cross: true
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}

      - name: Install musl tools (Linux x86_64)
        if: matrix.target == 'x86_64-unknown-linux-musl'
        run: sudo apt-get update && sudo apt-get install -y musl-tools

      - name: Install cross (Linux aarch64)
        if: matrix.cross
        run: cargo install cross --git https://github.com/cross-rs/cross

      - name: Build
        run: |
          if [ "${{ matrix.cross }}" = "true" ]; then
            cross build --release --target ${{ matrix.target }} --bin deep
          else
            cargo build --release --target ${{ matrix.target }} --bin deep
          fi

      - name: Strip binary
        run: |
          BIN="target/${{ matrix.target }}/release/deep"
          if command -v strip > /dev/null; then strip "$BIN" || true; fi

      - name: Create archive
        id: archive
        run: |
          VERSION="${GITHUB_REF_NAME}"
          NAME="deep-${VERSION}-${{ matrix.target }}"
          mkdir -p "dist/${NAME}"
          cp "target/${{ matrix.target }}/release/deep" "dist/${NAME}/"
          cp LICENSE README.md "dist/${NAME}/" 2>/dev/null || true
          cd dist
          tar czf "${NAME}.tar.gz" "${NAME}"
          cd ..
          echo "archive=dist/${NAME}.tar.gz" >> "$GITHUB_OUTPUT"
          echo "name=${NAME}" >> "$GITHUB_OUTPUT"

      - name: Upload artifact
        uses: actions/upload-artifact@v4
        with:
          name: ${{ steps.archive.outputs.name }}
          path: ${{ steps.archive.outputs.archive }}
          retention-days: 1

  release:
    name: Publish release
    needs: build
    runs-on: ubuntu-22.04
    steps:
      - uses: actions/checkout@v4

      - name: Download all artifacts
        uses: actions/download-artifact@v4
        with:
          path: dist
          merge-multiple: true

      - name: Generate checksums
        run: |
          cd dist
          sha256sum deep-*.tar.gz > "deep-${GITHUB_REF_NAME}-checksums.txt"

      - name: Create GitHub Release
        uses: softprops/action-gh-release@v2
        with:
          files: |
            dist/deep-*.tar.gz
            dist/deep-*-checksums.txt
          generate_release_notes: true
          fail_on_unmatched_files: true
```

---

## 5. Versiyon Yönetimi

**Source of truth:** Workspace root `Cargo.toml`'da `[workspace.package] version = "..."`. Üye crate'ler `version.workspace = true` ile bunu alır.

**Tag formatı:** `vX.Y.Z` (semver).

**Bump akışı:**
1. Workspace root `Cargo.toml`'da `[workspace.package] version` artır
2. `cargo build` → `Cargo.lock` güncellenir
3. `git commit -m "chore: bump version to v0.2.0"`
4. `git tag v0.2.0`
5. `git push && git push --tags`
6. Release workflow tetiklenir

**Pre-release:** `v0.2.0-rc.1` → GitHub Release "pre-release" işaretli.

---

## 6. Install Script

One-liner kurulum:
```bash
curl -fsSL cli.deepcanvas.studio/install.sh | sh
```

**Domain setup:**
- `cli.deepcanvas.studio` DNS:
  - Cloudflare Worker (statik script serve)
  - Veya GitHub Pages: repo `cli` branch'inde `install.sh`

**`install.sh` (repo root):**

```bash
#!/usr/bin/env sh
set -eu

REPO="deepcanvas-studio/deepcanvas-cli"
BIN_NAME="deep"

detect_target() {
    os="$(uname -s | tr '[:upper:]' '[:lower:]')"
    arch="$(uname -m)"
    case "$os" in
        darwin) os="apple-darwin" ;;
        linux)  os="unknown-linux-musl" ;;
        *) echo "unsupported OS: $os" >&2; exit 1 ;;
    esac
    case "$arch" in
        x86_64|amd64) arch="x86_64" ;;
        arm64|aarch64) arch="aarch64" ;;
        *) echo "unsupported arch: $arch" >&2; exit 1 ;;
    esac
    echo "${arch}-${os}"
}

detect_install_dir() {
    if [ -w /usr/local/bin ] 2>/dev/null; then
        echo "/usr/local/bin"
    elif [ -d "$HOME/.local/bin" ]; then
        echo "$HOME/.local/bin"
    else
        mkdir -p "$HOME/.local/bin"
        echo "$HOME/.local/bin"
    fi
}

get_latest_version() {
    curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
        | grep '"tag_name"' \
        | head -1 \
        | sed -E 's/.*"tag_name":[[:space:]]*"([^"]+)".*/\1/'
}

TARGET="$(detect_target)"
INSTALL_DIR="$(detect_install_dir)"
VERSION="${DEEP_VERSION:-$(get_latest_version)}"

if [ -z "$VERSION" ]; then
    echo "could not determine latest version" >&2
    exit 1
fi

ARCHIVE="deep-${VERSION}-${TARGET}.tar.gz"
URL="https://github.com/${REPO}/releases/download/${VERSION}/${ARCHIVE}"
CHECKSUMS_URL="https://github.com/${REPO}/releases/download/${VERSION}/deep-${VERSION}-checksums.txt"

echo "→ Installing deep ${VERSION} (${TARGET}) to ${INSTALL_DIR}/${BIN_NAME}"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

curl -fsSL "$URL" -o "$TMP/$ARCHIVE"
curl -fsSL "$CHECKSUMS_URL" -o "$TMP/checksums.txt" 2>/dev/null || true

# Verify checksum
if [ -f "$TMP/checksums.txt" ]; then
    cd "$TMP"
    if command -v sha256sum >/dev/null; then
        grep "  $ARCHIVE\$" checksums.txt | sha256sum -c -
    elif command -v shasum >/dev/null; then
        grep "  $ARCHIVE\$" checksums.txt | shasum -a 256 -c -
    fi
    cd - >/dev/null
fi

tar -xzf "$TMP/$ARCHIVE" -C "$TMP"
EXTRACTED="${TMP}/deep-${VERSION}-${TARGET}"
mv "${EXTRACTED}/${BIN_NAME}" "${INSTALL_DIR}/${BIN_NAME}"
chmod +x "${INSTALL_DIR}/${BIN_NAME}"

echo
echo "✓ Installed: ${INSTALL_DIR}/${BIN_NAME}"

case ":$PATH:" in
    *":${INSTALL_DIR}:"*)
        echo "  Run: deep --help"
        ;;
    *)
        echo
        echo "⚠ ${INSTALL_DIR} is not in your PATH."
        echo "  Add to shell config:"
        echo "    export PATH=\"${INSTALL_DIR}:\$PATH\""
        ;;
esac
```

**Env overrides:**
- `DEEP_VERSION=v0.2.0` — belirli versiyon kurulumu

**Hosting önerisi:** Cloudflare Worker basit, GitHub raw URL'i proxy eder. Veya GitHub Pages.

---

## 7. Homebrew Tap

**Ayrı repo:** `deepcanvas-studio/homebrew-tap`

```
homebrew-tap/
└── Formula/
    └── deep.rb
```

**`Formula/deep.rb`:**

```ruby
class Deep < Formula
  desc "DeepCanvas CLI — task and document context for coding agents"
  homepage "https://deepcanvas.studio"
  version "0.1.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/deepcanvas-studio/deepcanvas-cli/releases/download/v#{version}/deep-v#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "REPLACED_BY_CI_AARCH64_DARWIN"
    end
    on_intel do
      url "https://github.com/deepcanvas-studio/deepcanvas-cli/releases/download/v#{version}/deep-v#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "REPLACED_BY_CI_X86_64_DARWIN"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/deepcanvas-studio/deepcanvas-cli/releases/download/v#{version}/deep-v#{version}-aarch64-unknown-linux-musl.tar.gz"
      sha256 "REPLACED_BY_CI_AARCH64_LINUX"
    end
    on_intel do
      url "https://github.com/deepcanvas-studio/deepcanvas-cli/releases/download/v#{version}/deep-v#{version}-x86_64-unknown-linux-musl.tar.gz"
      sha256 "REPLACED_BY_CI_X86_64_LINUX"
    end
  end

  def install
    bin.install "deep"
  end

  test do
    assert_match "deep #{version}", shell_output("#{bin}/deep --version")
  end
end
```

**Kurulum:**
```bash
brew tap deepcanvas-studio/tap
brew install deep
# veya:
brew install deepcanvas-studio/tap/deep
```

### 7.1 Auto-Bump Job

`release.yml`'e ek job — yeni release sonrası formula bump PR'ı açar:

```yaml
  bump-homebrew:
    name: Bump Homebrew formula
    needs: release
    runs-on: ubuntu-22.04
    steps:
      - name: Checkout tap repo
        uses: actions/checkout@v4
        with:
          repository: deepcanvas-studio/homebrew-tap
          token: ${{ secrets.HOMEBREW_TAP_TOKEN }}
          path: tap

      - name: Download checksums
        run: |
          curl -fsSL \
            "https://github.com/deepcanvas-studio/deepcanvas-cli/releases/download/${GITHUB_REF_NAME}/deep-${GITHUB_REF_NAME}-checksums.txt" \
            -o checksums.txt
          cat checksums.txt

      - name: Update formula
        run: |
          VERSION="${GITHUB_REF_NAME#v}"
          FORMULA=tap/Formula/deep.rb

          AARCH64_DARWIN=$(grep "aarch64-apple-darwin.tar.gz" checksums.txt | awk '{print $1}')
          X86_64_DARWIN=$(grep "x86_64-apple-darwin.tar.gz" checksums.txt | awk '{print $1}')
          AARCH64_LINUX=$(grep "aarch64-unknown-linux-musl.tar.gz" checksums.txt | awk '{print $1}')
          X86_64_LINUX=$(grep "x86_64-unknown-linux-musl.tar.gz" checksums.txt | awk '{print $1}')

          # Python ile pozisyon-bağımsız replace (sed regex'i kırılgan)
          python3 << EOF
          import re
          path = "$FORMULA"
          content = open(path).read()
          content = re.sub(r'version ".+"', f'version "$VERSION"', content)

          # Each platform block — replace by URL match
          replacements = [
              ("aarch64-apple-darwin", "$AARCH64_DARWIN"),
              ("x86_64-apple-darwin", "$X86_64_DARWIN"),
              ("aarch64-unknown-linux-musl", "$AARCH64_LINUX"),
              ("x86_64-unknown-linux-musl", "$X86_64_LINUX"),
          ]
          for target, sha in replacements:
              pattern = rf'(url "[^"]*{re.escape(target)}[^"]*"\n\s+sha256 ")[a-f0-9_A-Z]+(")'
              content = re.sub(pattern, rf'\1{sha}\2', content)

          open(path, "w").write(content)
          EOF

      - name: Create PR
        run: |
          cd tap
          git config user.name "github-actions"
          git config user.email "actions@github.com"
          BRANCH="bump-deep-${GITHUB_REF_NAME}"
          git checkout -b "$BRANCH"
          git add Formula/deep.rb
          git commit -m "deep ${GITHUB_REF_NAME}"
          git push origin "$BRANCH"
          gh pr create --title "deep ${GITHUB_REF_NAME}" --body "Automated formula bump." --base main
        env:
          GH_TOKEN: ${{ secrets.HOMEBREW_TAP_TOKEN }}
```

**Secret gerekli:** `HOMEBREW_TAP_TOKEN` — `homebrew-tap` repo'suna write yetkili PAT. `deepcanvas-cli` repo settings → Secrets'ta.

---

## 8. self_update Pipeline Uyumu

`deep update` komutu (`cli-implementation-spec.md` §17) bu pipeline ile uyumlu olmalı:

| Config | Değer |
|---|---|
| `REPO_OWNER` | `deepcanvas-studio` |
| `REPO_NAME` | `deepcanvas-cli` |
| `bin_name` | `deep` |
| Asset pattern | `deep-vX.Y.Z-<target>.tar.gz` |
| Checksum file | `deep-vX.Y.Z-checksums.txt` |

Asset adlandırmayı değiştirirsen iki taraf birlikte güncellenmeli.

---

## 9. CI Workflow (Test/Lint)

**`.github/workflows/ci.yml`** — her PR/main push'unda:

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:

jobs:
  check:
    runs-on: ubuntu-22.04
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy
      - run: cargo fmt --check
      - run: cargo clippy --all-targets -- -D warnings
      - run: cargo test
      - run: cargo build --release
```

---

## 10. Release Öncesi Checklist

- [ ] `cargo test` passes
- [ ] `cargo clippy --all-targets -- -D warnings` temiz
- [ ] `cargo fmt --check` temiz
- [ ] Workspace `[workspace.package] version` doğru
- [ ] `CHANGELOG.md` güncel (varsa)
- [ ] Backend `cli-auth-api-spec.md` endpoint'leri prod'da
- [ ] `cli.deepcanvas.studio` DNS aktif
- [ ] `homebrew-tap` repo'su var
- [ ] `HOMEBREW_TAP_TOKEN` secret eklenmiş
- [ ] `git tag vX.Y.Z` + `git push --tags`

---

## 11. Acceptance Criteria

1. `v0.1.0` tag push'unda release workflow tetiklenir.
2. 4 target için arşivler ve checksums GitHub Release'e upload edilir.
3. `curl cli.deepcanvas.studio/install.sh | sh` macOS/Linux makinesinde `deep` binary'sini PATH'e kurar.
4. Install script SHA-256 checksum doğrular (mismatch'te exit 1).
5. `brew install deepcanvas-studio/tap/deep` çalışır (homebrew-tap repo'su mevcut + formula bump otomatik).
6. `deep update` Homebrew dışı kurulumda GitHub Releases'ten yeni versiyonu indirir + replace.
7. `deep --version` doğru semver gösterir.
8. `ci.yml` her PR'da test + clippy + fmt çalıştırır.
9. Release pre-release tag'lerde (`v0.2.0-rc.1`) GitHub'da "pre-release" işaretlenir.

---

## 12. Bu Faz Dışı (Sonraki Fazlar)

- **Windows desteği** — `x86_64-pc-windows-msvc` target, `.exe`, scoop manifest.
- **Apple notarization / codesigning** — şu an unsigned; kullanıcı `xattr -d com.apple.quarantine $(which deep)` ile bypass eder. Apple Developer Program + notarytool sonraki faz.
- **APT/RPM paketleri** — `cargo-deb`, `cargo-generate-rpm`.
- **Docker image** — `docker run deepcanvas/deep`.
- **Cosign / Sigstore** — supply chain signing.
- **Scoop manifest** (Windows package manager) — Windows gelene kadar yok.
- **Version rollback** — manuel `DEEP_VERSION=v0.1.0 curl ... | sh` yeterli.
- **Telemetry / update analytics** — bilgi toplamayız bu fazda.

---

## 13. İlk Release Süreci

İlk `v0.1.0` çıkartırken:

1. `deepcanvas-cli` repo'sunu GitHub'da oluştur (private veya public).
2. `cli-implementation-spec.md`'yi uygula, kodu commit et.
3. Bu spec'teki `.github/workflows/release.yml` + `ci.yml` ekle.
4. Repo root'a `install.sh` ekle.
5. `homebrew-tap` repo'sunu placeholder Formula ile oluştur.
6. `HOMEBREW_TAP_TOKEN` secret'ı ekle.
7. `cli.deepcanvas.studio` DNS + serving setup (Cloudflare Worker veya GitHub Pages).
8. `git tag v0.1.0 && git push --tags` → workflow tetiklenir.
9. Release oluştuğunda test:
   ```bash
   curl cli.deepcanvas.studio/install.sh | sh
   deep --version
   ```
10. `homebrew-tap` PR'ı merge → `brew install` test.

---

**Bu spec uygulandığında:** Deep CLI public dağıtım hazır. `deep update` self-update zinciri otomatik döner.