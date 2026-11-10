#!/usr/bin/env bash
set -uo pipefail

cd "$(dirname "$0")/.."

failed=0
skipped=()

run() {
  echo "==> $*"
  "$@"
  local status=$?
  if [ "$status" -ne 0 ]; then
    failed=1
  fi
  return 0
}

skip() {
  echo "SKIP: $*"
  skipped+=("$*")
}

has_npm_script() {
  local script="$1"
  node -e "const p=require('./package.json'); process.exit(p.scripts && p.scripts['$script'] ? 0 : 1)" >/dev/null 2>&1
}

if [ -f package.json ]; then
  if [ -d node_modules ]; then
    if has_npm_script "format:check"; then
      run npm run format:check
    else
      skip "npm run format:check (script missing)"
    fi
    if has_npm_script "lint"; then
      run npm run lint
    else
      skip "npm run lint (script missing)"
    fi
    if [ -f tsconfig.json ]; then
      if has_npm_script "typecheck"; then
        run npm run typecheck
      else
        skip "npm run typecheck (script missing)"
      fi
    else
      skip "npm run typecheck (no tsconfig.json)"
    fi
    if [ -d src ]; then
      if has_npm_script "test"; then
        run npm test
      else
        skip "npm test (script missing)"
      fi
    else
      skip "npm test (no src/)"
    fi
  else
    skip "frontend checks (node_modules missing)"
  fi
else
  skip "frontend checks (no package.json)"
fi

if [ -f src-tauri/Cargo.toml ]; then
  run cargo fmt --manifest-path src-tauri/Cargo.toml --check
  run cargo check --manifest-path src-tauri/Cargo.toml --all-targets
  run cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
  run cargo test --manifest-path src-tauri/Cargo.toml
else
  skip "Rust checks (no src-tauri/Cargo.toml)"
fi

if [ -d sidecar/jobspy_mcp/tests ]; then
  if command -v pytest >/dev/null 2>&1; then
    run pytest sidecar/jobspy_mcp/tests
  else
    skip "pytest (pytest not installed)"
  fi
else
  skip "pytest (no sidecar/jobspy_mcp/tests)"
fi

if [ "${#skipped[@]}" -gt 0 ]; then
  echo
  echo "Skipped checks (not claimed as passing):"
  for item in "${skipped[@]}"; do
    echo "  - $item"
  done
fi

if [ "$failed" -ne 0 ]; then
  echo
  echo "VERIFY FAILED"
  exit 1
fi

echo
echo "VERIFY PASSED"
