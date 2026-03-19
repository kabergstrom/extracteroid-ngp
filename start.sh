#!/usr/bin/env bash
# Start the newgameplus engine in dev mode with the extracteroid module.
# Kills any existing instance first, rebuilds, launches in background,
# and waits for the RPC server to be ready.

set -euo pipefail

if grep -qi microsoft /proc/version 2>/dev/null; then
    IS_WSL=1
else
    IS_WSL=0
fi

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ENGINE_DIR="$(cd "$SCRIPT_DIR/../newgameplus2" && pwd)"

if [ "$IS_WSL" = 1 ]; then
    CARGO="cargo.exe"
    WIN_ENGINE="$(wslpath -w "$ENGINE_DIR")"
    EXE="${WIN_ENGINE}\\target\\release\\newgameplus.exe"
    WIN_MODULE="$(wslpath -w "$SCRIPT_DIR")"
else
    CARGO="cargo"
    EXE="$ENGINE_DIR/target/release/newgameplus"
fi

LOG="/tmp/ngp_engine.log"

# Kill existing instance if running
if [ "$IS_WSL" = 1 ]; then
    powershell.exe -Command "Get-Process newgameplus -ErrorAction SilentlyContinue | Where-Object { \$_.Path -eq '$EXE' } | Stop-Process -Force" 2>/dev/null || true
    for i in $(seq 1 20); do
        if ! powershell.exe -Command "Get-Process newgameplus -ErrorAction SilentlyContinue | Where-Object { \$_.Path -eq '$EXE' }" 2>/dev/null | grep -q newgameplus; then
            break
        fi
        sleep 0.5
    done
else
    pkill -f "$EXE" 2>/dev/null || true
    sleep 0.5
fi

# Build engine (source-walk + engine binary)
(cd "$ENGINE_DIR" && $CARGO build -p source-walk --release) && \
(cd "$ENGINE_DIR" && $CARGO build -p newgameplus --release --no-default-features --features rafx-vulkan,profile-with-tracy) || exit 1

# Build the module
(cd "$SCRIPT_DIR" && $CARGO build) || exit 1

# Truncate log before launch
: > "$LOG"

# Launch engine in background with --module-dir pointing at extracteroid-ngp
if [ "$IS_WSL" = 1 ]; then
    cmd.exe /C "set RUST_LOG=info&& set RUST_BACKTRACE=1&& ${EXE} --module-crate extracteroid --module-dir ${WIN_MODULE} --target-dir ${WIN_MODULE}\\target --dev" > "$LOG" 2>&1 &
else
    RUST_LOG=info RUST_BACKTRACE=1 "$EXE" --module-crate extracteroid --module-dir "$SCRIPT_DIR" --target-dir "$SCRIPT_DIR/target" --dev > "$LOG" 2>&1 &
fi
ENGINE_PID=$!
echo "Engine PID: $ENGINE_PID, log: $LOG"

# Wait for RPC server to be ready
for i in $(seq 1 120); do
    if grep -q "RPC server listening" "$LOG" 2>/dev/null; then
        echo "Engine ready (RPC listening)"
        exit 0
    fi
    sleep 0.5
done
echo "Timed out waiting for engine to start"
exit 1
