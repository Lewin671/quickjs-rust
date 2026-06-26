#!/bin/sh
set -eu

repo="Lewin671/quickjs-rust"
version="latest"
install_dir="${QJS_RUST_INSTALL_DIR:-$HOME/.local/bin}"
bin_name="qjs-rust"

usage() {
  cat <<'USAGE'
Install the qjs-rust CLI from GitHub Releases.

Usage:
  install.sh [--upgrade] [--version <tag>] [--dir <path>]

Options:
  --upgrade       Install the selected release over any existing qjs-rust binary.
                  This is the default behavior, so rerunning the script updates it.
  --version <tag>  Release tag to install, for example v0.1.0-preview.4.
                   Defaults to the latest GitHub release.
  --dir <path>     Installation directory. Defaults to $HOME/.local/bin.
  -h, --help       Show this help.

Environment:
  QJS_RUST_INSTALL_DIR  Overrides the default installation directory.
USAGE
}

die() {
  printf 'install.sh: %s\n' "$*" >&2
  exit 1
}

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || die "required command not found: $1"
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --upgrade|--update)
      shift
      ;;
    --version)
      [ "$#" -ge 2 ] || die "--version requires a value"
      version="$2"
      shift 2
      ;;
    --dir)
      [ "$#" -ge 2 ] || die "--dir requires a value"
      install_dir="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      die "unknown argument: $1"
      ;;
  esac
done

need_cmd curl
need_cmd tar

case "$(uname -s)" in
  Darwin)
    os="apple-darwin"
    ;;
  Linux)
    os="unknown-linux-gnu"
    ;;
  *)
    die "unsupported operating system: $(uname -s)"
    ;;
esac

case "$(uname -m)" in
  arm64|aarch64)
    arch="aarch64"
    ;;
  x86_64|amd64)
    arch="x86_64"
    ;;
  *)
    die "unsupported CPU architecture: $(uname -m)"
    ;;
esac

artifact="qjs-rust-$arch-$os"
archive="$artifact.tar.gz"

if [ "$version" = "latest" ]; then
  base_url="https://github.com/$repo/releases/latest/download"
else
  base_url="https://github.com/$repo/releases/download/$version"
fi

tmp_dir="$(mktemp -d 2>/dev/null || mktemp -d -t qjs-rust-install)"
cleanup() {
  rm -rf "$tmp_dir"
}
trap cleanup EXIT HUP INT TERM

printf 'Downloading %s from %s...\n' "$archive" "$base_url"
curl -fsSL "$base_url/$archive" -o "$tmp_dir/$archive"
curl -fsSL "$base_url/SHA256SUMS" -o "$tmp_dir/SHA256SUMS"

(
  cd "$tmp_dir"
  checksum_line="$(grep "  $archive\$" SHA256SUMS)" || die "checksum not found for $archive"
  if command -v sha256sum >/dev/null 2>&1; then
    printf '%s\n' "$checksum_line" | sha256sum -c -
  elif command -v shasum >/dev/null 2>&1; then
    printf '%s\n' "$checksum_line" | shasum -a 256 -c -
  else
    die "required command not found: sha256sum or shasum"
  fi
)

tar -xzf "$tmp_dir/$archive" -C "$tmp_dir"

[ -x "$tmp_dir/$artifact/$bin_name" ] || die "archive did not contain executable $bin_name"

mkdir -p "$install_dir"
cp "$tmp_dir/$artifact/$bin_name" "$install_dir/$bin_name"
chmod +x "$install_dir/$bin_name"

printf 'Installed or updated %s at %s\n' "$bin_name" "$install_dir/$bin_name"
"$install_dir/$bin_name" --version

case ":$PATH:" in
  *":$install_dir:"*) ;;
  *)
    printf '\n%s is not on PATH. Add this to your shell profile:\n' "$install_dir"
    printf '  export PATH="%s:$PATH"\n' "$install_dir"
    ;;
esac
