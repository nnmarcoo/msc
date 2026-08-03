#!/usr/bin/env bash
set -euo pipefail

OS="$(uname -s)"

if command -v yt-dlp &>/dev/null; then
    echo "yt-dlp already installed: $(yt-dlp --version)"
    echo "Update it with: yt-dlp -U"
else
    if [ "$OS" = "Darwin" ]; then
        if ! command -v brew &>/dev/null; then
            echo "Homebrew not found: https://brew.sh"
            exit 1
        fi
        brew install yt-dlp

    elif [ "$OS" = "Linux" ]; then
        echo "yt-dlp not found. Install it with one of:"
        echo "  pipx install yt-dlp          (recommended: updates independently)"
        echo "  python3 -m pip install yt-dlp"
        echo "  sudo apt install yt-dlp      (often outdated)"
        echo "  sudo pacman -S yt-dlp"
        exit 1

    else
        echo "Unsupported OS: $OS. Use scripts/setup-explore.ps1 on Windows."
        exit 1
    fi
fi

echo ""
echo "Build with: cargo build --features explore"
echo "Downloading from YouTube is against its terms of service; the"
echo "explore feature is off by default for that reason."
