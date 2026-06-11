#!/bin/bash
# Bump the project version everywhere it is declared, commit and tag.
# Usage: ./scripts/bump-version.sh 0.2.0 [--no-git]
set -euo pipefail

if [ $# -lt 1 ]; then
  echo "Usage: $0 <new-version> [--no-git]" >&2
  echo "Example: $0 0.2.0" >&2
  exit 1
fi

NEW_VERSION="$1"
NO_GIT=false
[ "${2:-}" = "--no-git" ] && NO_GIT=true

if ! echo "$NEW_VERSION" | grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+$'; then
  echo "Error: Version must be SemVer (e.g. 0.2.0)." >&2
  exit 1
fi

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT_DIR"

echo "Bumping version to ${NEW_VERSION}..."

# Desktop client: package.json
python3 - "$NEW_VERSION" <<'PY'
import json, sys
version = sys.argv[1]
path = "apps/desktop-client/package.json"
with open(path) as f:
    data = json.load(f)
data["version"] = version
with open(path, "w") as f:
    json.dump(data, f, indent=2)
    f.write("\n")
print(f"  updated {path}")
PY

# Desktop client: tauri.conf.json
python3 - "$NEW_VERSION" <<'PY'
import json, sys
version = sys.argv[1]
path = "apps/desktop-client/src-tauri/tauri.conf.json"
with open(path) as f:
    data = json.load(f)
data["version"] = version
with open(path, "w") as f:
    json.dump(data, f, indent=2)
    f.write("\n")
print(f"  updated {path}")
PY

# Cargo.toml files: desktop client + server agent
for manifest in apps/desktop-client/src-tauri/Cargo.toml apps/server-agent/Cargo.toml; do
  sed -i -E "0,/^version = \"[0-9]+\.[0-9]+\.[0-9]+\"/s//version = \"${NEW_VERSION}\"/" "$manifest"
  echo "  updated $manifest"
done

# Refresh Cargo.lock entries for the bumped packages
if command -v cargo >/dev/null 2>&1; then
  cargo update -p server-agent -p appsdesktop-client --precise "$NEW_VERSION" 2>/dev/null || \
  cargo update -w --offline 2>/dev/null || true
fi

if [ "$NO_GIT" = false ]; then
  git add apps/desktop-client/package.json \
          apps/desktop-client/src-tauri/tauri.conf.json \
          apps/desktop-client/src-tauri/Cargo.toml \
          apps/server-agent/Cargo.toml \
          Cargo.lock 2>/dev/null || true
  git commit -m "chore: bump version to v${NEW_VERSION}"
  git tag "v${NEW_VERSION}"
  echo ""
  echo "Created commit and tag v${NEW_VERSION}."
  echo "Push with: git push origin main --tags"
  echo "Pushing the tag triggers the Release workflow which builds and publishes the update."
else
  echo "Skipped git commit/tag (--no-git)."
fi
