#!/usr/bin/env bash
# ==============================================================================
# Game Boy WASM Build Automation Script
# ==============================================================================
# Objective: Build Rust WASM target package for web frontend deployment.
# Output directory: web/pkg
# ==============================================================================

set -euo pipefail

# ANSI Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
BOLD='\033[1m'
NC='\033[0m'

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WEB_DIR="${PROJECT_ROOT}/web"
PKG_DIR="${WEB_DIR}/pkg"

echo -e "${BOLD}${BLUE}====================================================${NC}"
echo -e "${BOLD}${BLUE}       Game Boy Emulator WASM Build Pipeline        ${NC}"
echo -e "${BOLD}${BLUE}====================================================${NC}"

# 1. Check wasm-pack installation
if ! command -v wasm-pack &>/dev/null; then
    echo -e "${RED}Error: wasm-pack is not installed or not in PATH.${NC}" >&2
    echo -e "${YELLOW}To install wasm-pack, run:${NC}" >&2
    echo -e "  curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh" >&2
    echo -e "  or: cargo install wasm-pack" >&2
    exit 1
fi

# 2. Execute wasm-pack build
echo -e "\n${BOLD}Step 1: Compiling Rust to WASM (--target web --out-dir web/pkg)${NC}"
cd "${PROJECT_ROOT}"

wasm-pack build --target web --out-dir "${PKG_DIR}"

# 3. Verify Artifact Integrity
echo -e "\n${BOLD}Step 2: Verifying Generated WASM Package Assets${NC}"

WASM_FILE="${PKG_DIR}/gb_emulator_bg.wasm"
JS_FILE="${PKG_DIR}/gb_emulator.js"

ERRORS=0

if [[ -f "${WASM_FILE}" ]] && [[ -s "${WASM_FILE}" ]]; then
    SIZE=$(du -h "${WASM_FILE}" | cut -f1)
    echo -e "[ ${GREEN}OK${NC} ] ${WASM_FILE#${PROJECT_ROOT}/} (${SIZE})"
else
    echo -e "[ ${RED}FAIL${NC} ] WASM binary missing or empty: ${WASM_FILE}" >&2
    ERRORS=$((ERRORS + 1))
fi

if [[ -f "${JS_FILE}" ]] && [[ -s "${JS_FILE}" ]]; then
    SIZE=$(du -h "${JS_FILE}" | cut -f1)
    echo -e "[ ${GREEN}OK${NC} ] ${JS_FILE#${PROJECT_ROOT}/} (${SIZE})"
else
    echo -e "[ ${RED}FAIL${NC} ] JS glue code missing or empty: ${JS_FILE}" >&2
    ERRORS=$((ERRORS + 1))
fi

if [[ "${ERRORS}" -eq 0 ]]; then
    echo -e "\n${BOLD}${GREEN}SUCCESS: WASM package built successfully in web/pkg!${NC}\n"
    exit 0
else
    echo -e "\n${BOLD}${RED}FAILURE: WASM build verification failed with ${ERRORS} error(s).${NC}\n" >&2
    exit 1
fi
