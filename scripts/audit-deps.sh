#!/usr/bin/env bash
# Copyright (c) 2026 Meng Zhuo
# SPDX-License-Identifier: BSD-3-Clause
#
# Supply-chain checks for the workspace.  The project has a single runtime
# dependency (libm); this catches accidental additions and RustSec advisories.
#
#   ./scripts/audit-deps.sh
#
# Set REQUIRE_AUDIT=1 to fail when the tools are not installed (CI does this
# only if the job installs them first).

set -euo pipefail

cd "$(dirname "$0")/.."

if command -v cargo-deny >/dev/null 2>&1; then
    cargo deny check
elif [[ ${REQUIRE_AUDIT:-0} == 1 ]]; then
    echo "error: cargo-deny is not installed" >&2
    exit 1
else
    echo "audit-deps: cargo-deny not installed; skipping advisory/license/source checks"
fi

if command -v cargo-audit >/dev/null 2>&1; then
    cargo audit
elif [[ ${REQUIRE_AUDIT:-0} == 1 ]]; then
    echo "error: cargo-audit is not installed" >&2
    exit 1
else
    echo "audit-deps: cargo-audit not installed; skipping RustSec check"
fi

echo "audit-deps: done"
