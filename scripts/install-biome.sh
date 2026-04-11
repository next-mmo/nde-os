#!/usr/bin/env sh

OS="$(uname -s)"
ARCH="$(uname -m)"

echo "Fetching latest Biome version..."
if command -v npm >/dev/null 2>&1; then
    VERSION="$(npm view @biomejs/biome version)"
else
    # Fallback to curl with grep if npm isn't available
    VERSION="$(curl -s "https://registry.npmjs.org/@biomejs/biome/latest" | grep -o '"version":"[^"]*"' | head -1 | cut -d'"' -f4)"
fi

if [ -z "$VERSION" ]; then
    echo "Could not fetch latest version, falling back to 2.4.11..."
    VERSION="2.4.11"
fi

echo "Detected OS: $OS, Architecture: $ARCH"
echo "Installing Biome version: $VERSION"

BIOME_URL="https://github.com/biomejs/biome/releases/download/@biomejs/biome@${VERSION}"

case "$OS" in
    Darwin)
        if [ "$ARCH" = "arm64" ]; then
            curl -L "$BIOME_URL/biome-darwin-arm64" -o biome
        else
            curl -L "$BIOME_URL/biome-darwin-x64" -o biome
        fi
        chmod +x biome
        echo "Biome installed for macOS"
        ;;
    Linux)
        if [ "$ARCH" = "aarch64" ]; then
            curl -L "$BIOME_URL/biome-linux-arm64" -o biome
        else
            curl -L "$BIOME_URL/biome-linux-x64" -o biome
        fi
        chmod +x biome
        echo "Biome installed for Linux"
        ;;
    MINGW*|CYGWIN*|MSYS*)
        curl -L "$BIOME_URL/biome-win32-x64.exe" -o biome.exe
        echo "Biome installed for Windows"
        ;;
    *)
        echo "Unsupported OS: $OS"
        exit 1
        ;;
esac