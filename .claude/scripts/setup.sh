#!/usr/bin/env bash
# Claude Code setup script — installs Rust cargo tools via cargo-binstall.
# This runs automatically at the start of each Claude Code session.

set -euo pipefail

echo "==> Installing Rust toolchain components..."
rustup show active-toolchain || rustup default stable

# Install cargo-binstall if not already present
if ! command -v cargo-binstall &>/dev/null; then
    echo "==> Installing cargo-binstall..."
    curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash
fi

# Tools required by this project (see CLAUDE.md)
TOOLS=(
    cargo-nextest      # Preferred test runner
    cargo-deny         # Dependency audit (licenses, advisories, bans)
    cargo-llvm-cov     # Code coverage
    cargo-machete      # Unused dependency detection
)

echo "==> Installing cargo tools via cargo-binstall..."
for tool in "${TOOLS[@]}"; do
    # Strip inline comments
    tool="${tool%%#*}"
    tool="${tool// /}"
    if ! command -v "$tool" &>/dev/null; then
        echo "    Installing $tool..."
        cargo binstall --no-confirm "$tool"
    else
        echo "    $tool already installed."
    fi
done

# Conditional tools — installed only when their framework/library is detected
REPO_ROOT="$(git rev-parse --show-toplevel)"

if grep -q 'dioxus' "$REPO_ROOT/Cargo.toml" 2>/dev/null; then
    # Extract dioxus version from Cargo.toml (handles both 'dioxus = "0.7"' and 'dioxus = { version = "0.7", ... }')
    DIOXUS_VERSION=$(grep -oP 'dioxus\s*=\s*(?:"([^"]+)"|\{[^}]*version\s*=\s*"([^"]+)")' "$REPO_ROOT/Cargo.toml" | grep -oP '"\K[^"]+' | head -1)

    NEEDS_INSTALL=false
    if ! command -v dx &>/dev/null; then
        NEEDS_INSTALL=true
    elif [ -n "$DIOXUS_VERSION" ]; then
        INSTALLED_VERSION=$(dx --version 2>/dev/null | grep -oP '[0-9]+\.[0-9]+\.[0-9]+' || echo "")
        if [ "$INSTALLED_VERSION" != "$DIOXUS_VERSION" ]; then
            echo "==> Dioxus CLI version mismatch (installed: ${INSTALLED_VERSION:-unknown}, needed: $DIOXUS_VERSION)"
            NEEDS_INSTALL=true
        fi
    fi

    if [ "$NEEDS_INSTALL" = true ] && [ -n "$DIOXUS_VERSION" ]; then
        echo "==> Dioxus detected in Cargo.toml — installing dioxus-cli v${DIOXUS_VERSION}..."
        curl -sSL https://dioxus.dev/install.sh | sh -s -- "dx-v${DIOXUS_VERSION}"
    elif [ "$NEEDS_INSTALL" = true ]; then
        echo "==> Dioxus detected in Cargo.toml — installing dioxus-cli (latest)..."
        curl -sSL https://dioxus.dev/install.sh | bash
    else
        echo "==> Dioxus detected — dx already installed (v${INSTALLED_VERSION})."
    fi
fi

# Install pre-commit hook (idempotent — uses symlink)
echo "==> Installing pre-commit hook..."
ln -sf "$REPO_ROOT/scripts/pre-commit" "$REPO_ROOT/.git/hooks/pre-commit"
chmod +x "$REPO_ROOT/scripts/pre-commit"

echo "==> Setup complete."
