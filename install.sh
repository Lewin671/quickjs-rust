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

download() {
  url="$1"
  output="$2"
  label="$3"
  if ! curl -fsSL "$url" -o "$output"; then
    die "failed to download $label from $url"
  fi
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
      die "unknown argument: $1. Run with --help for usage."
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
install_tmp=""
cleanup() {
  rm -rf "$tmp_dir"
  if [ -n "$install_tmp" ]; then
    rm -f "$install_tmp"
  fi
}
trap cleanup EXIT HUP INT TERM

mkdir -p "$install_dir" || die "failed to create installation directory: $install_dir"
[ -d "$install_dir" ] || die "installation path is not a directory: $install_dir"
[ -w "$install_dir" ] || die "installation directory is not writable: $install_dir"

printf 'Downloading %s from %s...\n' "$archive" "$base_url"
download "$base_url/$archive" "$tmp_dir/$archive" "release asset $archive"
download "$base_url/SHA256SUMS" "$tmp_dir/SHA256SUMS" "release checksums"

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

install_tmp="$install_dir/.$bin_name.tmp.$$"
cp "$tmp_dir/$artifact/$bin_name" "$install_tmp" || die "failed to copy $bin_name into $install_dir"
chmod +x "$install_tmp" || die "failed to mark $bin_name executable"
mv "$install_tmp" "$install_dir/$bin_name" || die "failed to replace $install_dir/$bin_name"
install_tmp=""

printf 'Installed or updated %s at %s\n' "$bin_name" "$install_dir/$bin_name"
"$install_dir/$bin_name" --version

case ":$PATH:" in
  *":$install_dir:"*) ;;
  *)
    printf '\n%s is not on PATH. Add this to your shell profile:\n' "$install_dir"
    printf '  export PATH="%s:$PATH"\n' "$install_dir"
    ;;
esac
