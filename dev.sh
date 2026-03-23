#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────
#  AI Launcher — Full Development Script
#  Starts both the Rust API server and the Tauri desktop app.
#
#  Usage:
#    ./dev.sh              # start server + tauri desktop
#    ./dev.sh server       # start only the API server
#    ./dev.sh desktop      # start only the Tauri desktop app
#    ./dev.sh vite         # start only the Vite dev server (no Tauri)
#
#  Press Ctrl+C to stop all processes.
# ─────────────────────────────────────────────────────────────
set -euo pipefail

# ── Resolve project root (where this script lives) ──────────
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$SCRIPT_DIR"
DESKTOP_DIR="$PROJECT_ROOT/desktop"

# ── Colors ──────────────────────────────────────────────────
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
BOLD='\033[1m'
DIM='\033[2m'
RESET='\033[0m'

# ── PIDs to track ───────────────────────────────────────────
SERVER_PID=""
TAURI_PID=""
VITE_PID=""

# ── Cleanup on exit ─────────────────────────────────────────
cleanup() {
    echo ""
    echo -e "${YELLOW}⏹  Shutting down...${RESET}"

    # Kill child processes
    for pid in $SERVER_PID $TAURI_PID $VITE_PID; do
        if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
            echo -e "${DIM}   Stopping PID $pid${RESET}"
            kill "$pid" 2>/dev/null || true
        fi
    done

    # Wait briefly, then force-kill stragglers
    sleep 1
    for pid in $SERVER_PID $TAURI_PID $VITE_PID; do
        if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
            kill -9 "$pid" 2>/dev/null || true
        fi
    done

    echo -e "${GREEN}✓  All processes stopped.${RESET}"
    exit 0
}

trap cleanup SIGINT SIGTERM EXIT

# ── Kill process occupying a port ───────────────────────────
kill_port() {
    local port=$1
    local pids=""

    if [[ "$OSTYPE" == "msys" || "$OSTYPE" == "cygwin" || "$OSTYPE" == "win32" ]] || command -v taskkill &>/dev/null; then
        # Windows (Git Bash / MSYS2 / Cygwin)
        pids=$(netstat -ano 2>/dev/null | grep ":${port} " | grep 'LISTENING' | awk '{print $5}' | sort -u | grep -v '^0$' || true)
        if [ -n "$pids" ]; then
            for pid in $pids; do
                echo -e "${YELLOW}⚡ Killing process on port $port ${DIM}(PID $pid)${RESET}"
                taskkill //F //PID "$pid" 2>/dev/null || true
            done
        fi
    else
        # Linux / macOS
        pids=$(lsof -ti :"$port" 2>/dev/null || true)
        if [ -n "$pids" ]; then
            for pid in $pids; do
                echo -e "${YELLOW}⚡ Killing process on port $port ${DIM}(PID $pid)${RESET}"
                kill -9 "$pid" 2>/dev/null || true
            done
        fi
    fi
}

# ── Free required ports ─────────────────────────────────────
free_ports() {
    local ports=("$@")
    local killed=0

    for port in "${ports[@]}"; do
        # Check if port is in use before trying to kill
        if [[ "$OSTYPE" == "msys" || "$OSTYPE" == "cygwin" || "$OSTYPE" == "win32" ]] || command -v taskkill &>/dev/null; then
            if netstat -ano 2>/dev/null | grep ":${port} " | grep -q 'LISTENING'; then
                kill_port "$port"
                killed=1
            fi
        else
            if lsof -ti :"$port" &>/dev/null; then
                kill_port "$port"
                killed=1
            fi
        fi
    done

    if [ $killed -eq 1 ]; then
        echo -e "${DIM}   Waiting 1s for ports to free...${RESET}"
        sleep 1
        echo ""
    fi
}

# ── Dependency checks ───────────────────────────────────────
check_deps() {
    local missing=0

    if ! command -v cargo &>/dev/null; then
        echo -e "${RED}✗  cargo not found. Install Rust: https://rustup.rs${RESET}"
        missing=1
    fi

    if ! command -v node &>/dev/null; then
        echo -e "${RED}✗  node not found. Install Node.js: https://nodejs.org${RESET}"
        missing=1
    fi

    if ! command -v pnpm &>/dev/null; then
        echo -e "${RED}✗  pnpm not found. Install pnpm: https://pnpm.io/installation${RESET}"
        missing=1
    fi

    if [ $missing -eq 1 ]; then
        echo -e "${RED}Aborting — fix missing dependencies above.${RESET}"
        exit 1
    fi
}

# ── Ensure desktop node_modules exist ───────────────────────
ensure_node_modules() {
    if [ ! -d "$DESKTOP_DIR/node_modules" ]; then
        echo -e "${CYAN}📦  Installing desktop dependencies...${RESET}"
        (cd "$DESKTOP_DIR" && pnpm install)
        echo ""
    fi
}

# ── Banner ──────────────────────────────────────────────────
banner() {
    echo ""
    echo -e "${BOLD}${CYAN}  ╔═══════════════════════════════════════════════╗${RESET}"
    echo -e "${BOLD}${CYAN}  ║${RESET}  ${BOLD}🚀 AI Launcher — Development Mode${RESET}             ${BOLD}${CYAN}║${RESET}"
    echo -e "${BOLD}${CYAN}  ║${RESET}  ${DIM}v0.2.0 • Rust + Svelte 5 + Tauri 2${RESET}           ${BOLD}${CYAN}║${RESET}"
    echo -e "${BOLD}${CYAN}  ╚═══════════════════════════════════════════════╝${RESET}"
    echo ""
}

# ── Start API Server ───────────────────────────────────────
start_server() {
    echo -e "${GREEN}▶  Starting API server ${DIM}(cargo run -p ai-launcher-server)${RESET}"
    echo -e "${DIM}   → http://localhost:8080${RESET}"
    echo -e "${DIM}   → http://localhost:8080/swagger-ui/${RESET}"
    echo ""

    (cd "$PROJECT_ROOT" && cargo run -p ai-launcher-server 2>&1 | while IFS= read -r line; do
        echo -e "${BLUE}[server]${RESET} $line"
    done) &
    SERVER_PID=$!
}

# ── Start Tauri Desktop (includes Vite dev server) ──────────
start_tauri() {
    echo -e "${GREEN}▶  Starting Tauri desktop ${DIM}(pnpm tauri dev)${RESET}"
    echo -e "${DIM}   → Vite: http://localhost:5174${RESET}"
    echo -e "${DIM}   → Tauri window will open automatically${RESET}"
    echo ""

    (cd "$DESKTOP_DIR" && pnpm tauri dev 2>&1 | while IFS= read -r line; do
        echo -e "${CYAN}[tauri]${RESET}  $line"
    done) &
    TAURI_PID=$!
}

# ── Start Vite only (no Tauri window) ───────────────────────
start_vite() {
    echo -e "${GREEN}▶  Starting Vite dev server ${DIM}(pnpm dev)${RESET}"
    echo -e "${DIM}   → http://localhost:5174${RESET}"
    echo ""

    (cd "$DESKTOP_DIR" && pnpm dev 2>&1 | while IFS= read -r line; do
        echo -e "${CYAN}[vite]${RESET}   $line"
    done) &
    VITE_PID=$!
}

# ── Wait for processes ──────────────────────────────────────
wait_for_all() {
    echo -e "${DIM}──────────────────────────────────────────────────${RESET}"
    echo -e "${YELLOW}   Press Ctrl+C to stop all processes${RESET}"
    echo -e "${DIM}──────────────────────────────────────────────────${RESET}"
    echo ""

    # Wait for any child to exit
    wait
}

# ── Main ────────────────────────────────────────────────────
MODE="${1:-all}"

check_deps
banner

case "$MODE" in
    server)
        echo -e "${BOLD}  Mode: ${GREEN}server only${RESET}"
        echo ""
        free_ports 8080
        start_server
        wait_for_all
        ;;
    desktop|tauri)
        echo -e "${BOLD}  Mode: ${CYAN}Tauri desktop only${RESET}"
        echo ""
        free_ports 5174
        ensure_node_modules
        start_tauri
        wait_for_all
        ;;
    vite|frontend)
        echo -e "${BOLD}  Mode: ${CYAN}Vite frontend only${RESET}"
        echo ""
        free_ports 5174
        ensure_node_modules
        start_vite
        wait_for_all
        ;;
    all|"")
        echo -e "${BOLD}  Mode: ${GREEN}server${RESET} + ${CYAN}Tauri desktop${RESET}"
        echo ""
        free_ports 8080 5174
        ensure_node_modules
        start_server

        # Give the server a moment to begin binding
        echo -e "${DIM}   Waiting 3s for server to start...${RESET}"
        sleep 3

        start_tauri
        wait_for_all
        ;;
    *)
        echo -e "${RED}Unknown mode: $MODE${RESET}"
        echo ""
        echo "Usage: ./dev.sh [server|desktop|vite|all]"
        echo ""
        echo "  all       Start API server + Tauri desktop (default)"
        echo "  server    Start only the Rust API server on :8080"
        echo "  desktop   Start only the Tauri desktop app"
        echo "  vite      Start only the Vite frontend (no Tauri)"
        exit 1
        ;;
esac
