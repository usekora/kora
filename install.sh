#!/bin/bash
set -euo pipefail

REPO="kora-ai/kora"
BINARY="kora"

get_arch() {
    local arch
    arch=$(uname -m)
    case "$arch" in
        x86_64|amd64) echo "x86_64" ;;
        arm64|aarch64) echo "aarch64" ;;
        *) echo "Unsupported architecture: $arch" >&2; exit 1 ;;
    esac
}

get_os() {
    local os
    os=$(uname -s | tr '[:upper:]' '[:lower:]')
    case "$os" in
        linux) echo "unknown-linux-gnu" ;;
        darwin) echo "apple-darwin" ;;
        *) echo "Unsupported OS: $os" >&2; exit 1 ;;
    esac
}

main() {
    local arch os target version download_url tmp_dir

    arch=$(get_arch)
    os=$(get_os)
    target="${arch}-${os}"

    echo "Detecting platform: ${target}"

    version=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
        | grep '"tag_name"' | head -1 | cut -d'"' -f4)

    if [ -z "$version" ]; then
        echo "Failed to fetch latest version" >&2
        exit 1
    fi

    echo "Installing kora ${version}..."

    download_url="https://github.com/${REPO}/releases/download/${version}/kora-${target}.tar.gz"

    tmp_dir=$(mktemp -d)
    trap 'rm -rf "$tmp_dir"' EXIT

    curl -fsSL "$download_url" | tar xz -C "$tmp_dir"

    local install_dir="/usr/local/bin"
    if [ ! -w "$install_dir" ]; then
        echo "Installing to ${install_dir} (requires sudo)..."
        sudo mv "${tmp_dir}/${BINARY}" "${install_dir}/${BINARY}"
    else
        mv "${tmp_dir}/${BINARY}" "${install_dir}/${BINARY}"
    fi

    chmod +x "${install_dir}/${BINARY}"

    echo "kora ${version} installed to ${install_dir}/${BINARY}"
}

main
