#!/bin/bash

set -euo pipefail

if [[ "$(uname -s)" != "Darwin" ]]; then
    echo "macOS smoke test requires Darwin" >&2
    exit 1
fi

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
tmpdir="$(mktemp -d)"
label="com.local.cronlaunch-smoke-$$"
handler="$tmpdir/handler.sh"
marker="$tmpdir/marker"
label_hint="$tmpdir/cronlaunch-smoke-$$.label"
binary="$repo_root/target/debug/loginl"

cleanup() {
    HOME="$tmpdir" "$binary" --remove "$label" >/dev/null 2>&1 || true
    rm -rf "$tmpdir"
}
trap cleanup EXIT

cat >"$handler" <<EOF
#!/bin/sh
touch "$marker"
EOF
chmod 755 "$handler"
touch "$label_hint"

cargo build --bin loginl --locked --manifest-path "$repo_root/Cargo.toml"
HOME="$tmpdir" "$binary" "$label_hint" -- "$handler"

plist="$tmpdir/Library/LaunchAgents/$label.plist"
plutil -lint "$plist"

for _ in {1..20}; do
    [[ -f "$marker" ]] && break
    sleep 1
done

[[ -f "$marker" ]] || { echo "RunAtLoad agent did not execute" >&2; exit 1; }
launchctl print "gui/$(id -u)/$label" >/dev/null
