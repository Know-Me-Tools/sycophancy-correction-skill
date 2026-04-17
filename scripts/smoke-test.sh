#!/usr/bin/env bash
# Smoke test for sycophancy-correction MCP server.
# Verifies the server starts, responds to a minimal request, and exposes expected tools.
# Exit codes: 0 = pass, non-zero = fail with specific diagnostic.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR/.."

# Resolve binary — prefer installed, fall back to target/release
if command -v sycophancy-correction >/dev/null 2>&1; then
    BIN="sycophancy-correction"
elif [ -x "target/release/sycophancy-correction" ]; then
    BIN="target/release/sycophancy-correction"
else
    echo "❌ sycophancy-correction binary not found."
    echo "   Run: cargo build --release  OR  ./scripts/check-prerequisites.sh --install (from skill pack root)"
    exit 1
fi

echo "🔍 Smoke test: $BIN"

if [ -z "${ANTHROPIC_API_KEY:-}" ]; then
    echo "⚠️  ANTHROPIC_API_KEY not set. Detection works; correction will return stubbed output."
fi

# Resolve timeout command (macOS Homebrew installs to /opt/homebrew/bin)
TIMEOUT_CMD=""
for t in timeout gtimeout /opt/homebrew/bin/timeout /usr/local/bin/timeout; do
    if command -v "$t" >/dev/null 2>&1; then
        TIMEOUT_CMD="$t"
        break
    fi
done

# Use a FIFO so we can send messages with small delays between them.
# The MCP server processes messages asynchronously; sending all at once causes
# the tools/list request to arrive before the initialized notification is handled.
FIFO=$(mktemp -u /tmp/mcp_smoke_XXXXX)
mkfifo "$FIFO"
trap "rm -f $FIFO" EXIT

(
    printf '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"smoke-test","version":"0.0.0"}}}\n'
    sleep 0.3
    printf '{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}\n'
    sleep 0.2
    printf '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}\n'
    sleep 2
) > "$FIFO" &

if [ -n "$TIMEOUT_CMD" ]; then
    RESPONSE=$("$TIMEOUT_CMD" 30 "$BIN" --config skill.toml < "$FIFO" 2>/dev/null) || {
        echo "❌ Server did not respond within 30s."
        exit 2
    }
else
    echo "⚠️  timeout command not found — running without time limit (Ctrl-C to abort)"
    RESPONSE=$("$BIN" --config skill.toml < "$FIFO" 2>/dev/null) || {
        echo "❌ Server failed to respond."
        exit 2
    }
fi

if echo "$RESPONSE" | grep -q '"detect_sycophancy"'; then
    echo "✅ Server responded and exposes detect_sycophancy tool"
else
    echo "❌ detect_sycophancy tool not found in tools/list"
    echo "   Response excerpt: $(echo "$RESPONSE" | head -2)"
    exit 3
fi

if echo "$RESPONSE" | grep -q '"analyze_reflect_phase"'; then
    echo "✅ analyze_reflect_phase tool present"
else
    echo "⚠️  analyze_reflect_phase missing — Reflect integration falls back to detect_sycophancy"
fi

echo ""
echo "✅ Smoke test passed"
exit 0
