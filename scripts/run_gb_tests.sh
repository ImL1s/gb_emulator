#!/usr/bin/env bash
# ==============================================================================
# Game Boy (LR35902) Test ROM Automation Runner Script
# ==============================================================================
# Objective: Download Blargg test ROMs, execute emulator in headless mode,
# verify serial port ASCII output ("Passed" / "Failed"), and return exit code 0.
# ==============================================================================

set -euo pipefail

# ANSI Color Codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
BOLD='\033[1m'
NC='\033[0m' # No Color

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ROM_DIR="${PROJECT_ROOT}/tests/roms"
INDIVIDUAL_DIR="${ROM_DIR}/individual"
TARGET_BIN="${PROJECT_ROOT}/target/release/gb_emulator"
RAW_BASE_URL="https://raw.githubusercontent.com/retrio/gb-test-roms/master"

DOWNLOAD_ONLY=false
HEADLESS=true
CUSTOM_TIMEOUT=""

# Parse CLI options
while [[ $# -gt 0 ]]; do
  case "$1" in
    --download-only)
      DOWNLOAD_ONLY=true
      shift
      ;;
    --headless)
      HEADLESS=true
      shift
      ;;
    --timeout)
      if [[ -n "${2:-}" ]] && [[ "$2" =~ ^[0-9]+$ ]]; then
        CUSTOM_TIMEOUT="$2"
        shift 2
      else
        echo -e "${RED}Error: --timeout requires a numeric argument${NC}" >&2
        exit 1
      fi
      ;;
    --help|-h)
      echo -e "${BOLD}Game Boy Test Harness Runner Usage:${NC}"
      echo -e "  $0 [OPTIONS]"
      echo -e ""
      echo -e "${BOLD}Options:${NC}"
      echo -e "  --download-only   Download and verify Blargg test ROMs without running emulator"
      echo -e "  --headless        Run emulator in headless mode (default)"
      echo -e "  --timeout <N>     Override per-ROM execution timeout with <N> seconds"
      echo -e "  --help, -h        Show this help message"
      exit 0
      ;;
    *)
      echo -e "${RED}Error: Unknown option $1${NC}" >&2
      echo -e "Use $0 --help for usage information." >&2
      exit 1
      ;;
  esac
done

# Ensure ROM directories exist
mkdir -p "${ROM_DIR}"
mkdir -p "${INDIVIDUAL_DIR}"

echo -e "${BOLD}${BLUE}====================================================${NC}"
echo -e "${BOLD}${BLUE}   Game Boy Emulator Test Harness Runner (Blargg)   ${NC}"
echo -e "${BOLD}${BLUE}====================================================${NC}"

# Function to fetch and cache ROMs
fetch_rom() {
    local remote_path="$1"
    local local_path="$2"

    if [[ -f "${local_path}" ]] && [[ -s "${local_path}" ]]; then
        echo -e "[ ${GREEN}CACHED${NC} ] ${local_path#${PROJECT_ROOT}/}"
        return 0
    fi

    echo -e "[ ${YELLOW}DOWNLOADING${NC} ] ${remote_path} -> ${local_path#${PROJECT_ROOT}/} ..."
    local url="${RAW_BASE_URL}/${remote_path}"
    
    if command -v curl &>/dev/null; then
        curl -sSL --fail "${url}" -o "${local_path}"
    elif command -v wget &>/dev/null; then
        wget -q "${url}" -O "${local_path}"
    else
        echo -e "${RED}Error: Neither curl nor wget is installed.${NC}" >&2
        exit 1
    fi

    if [[ ! -s "${local_path}" ]]; then
        echo -e "${RED}Error: Downloaded file ${local_path} is empty or missing.${NC}" >&2
        exit 1
    fi
}

echo -e "\n${BOLD}Step 1: Downloading & Verifying Blargg Test ROM Assets${NC}"

# 13 Blargg Test ROMs
fetch_rom "cpu_instrs/cpu_instrs.gb" "${ROM_DIR}/cpu_instrs.gb"
fetch_rom "instr_timing/instr_timing.gb" "${ROM_DIR}/instr_timing.gb"
fetch_rom "cpu_instrs/individual/01-special.gb" "${INDIVIDUAL_DIR}/01-special.gb"
fetch_rom "cpu_instrs/individual/02-interrupts.gb" "${INDIVIDUAL_DIR}/02-interrupts.gb"
fetch_rom "cpu_instrs/individual/03-op%20sp,hl.gb" "${INDIVIDUAL_DIR}/03-op sp,hl.gb"
fetch_rom "cpu_instrs/individual/04-op%20r,imm.gb" "${INDIVIDUAL_DIR}/04-op r,imm.gb"
fetch_rom "cpu_instrs/individual/05-op%20rp.gb" "${INDIVIDUAL_DIR}/05-op rp.gb"
fetch_rom "cpu_instrs/individual/06-ld%20r,r.gb" "${INDIVIDUAL_DIR}/06-ld r,r.gb"
fetch_rom "cpu_instrs/individual/07-jr,jp,call,ret,rst.gb" "${INDIVIDUAL_DIR}/07-jr,jp,call,ret,rst.gb"
fetch_rom "cpu_instrs/individual/08-misc%20instrs.gb" "${INDIVIDUAL_DIR}/08-misc instrs.gb"
fetch_rom "cpu_instrs/individual/09-op%20r,r.gb" "${INDIVIDUAL_DIR}/09-op r,r.gb"
fetch_rom "cpu_instrs/individual/10-bit%20ops.gb" "${INDIVIDUAL_DIR}/10-bit ops.gb"
fetch_rom "cpu_instrs/individual/11-op%20a,(hl).gb" "${INDIVIDUAL_DIR}/11-op a,(hl).gb"

echo -e "${GREEN}All 13 test ROMs downloaded and verified successfully.${NC}"

if [[ "${DOWNLOAD_ONLY}" == true ]]; then
    echo -e "\n${BOLD}${GREEN}DOWNLOAD COMPLETE: 13 / 13 test ROMs ready in cache.${NC}"
    echo -e "${BLUE}Exiting cleanly (--download-only mode).${NC}\n"
    exit 0
fi

# 2. Build Emulator Release Binary
echo -e "\n${BOLD}Step 2: Building Emulator Release Binary (${TARGET_BIN})${NC}"
if [[ -f "${PROJECT_ROOT}/Cargo.toml" ]]; then
    if cargo build --release --manifest-path "${PROJECT_ROOT}/Cargo.toml"; then
        echo -e "${GREEN}Cargo release build succeeded.${NC}"
    else
        echo -e "${RED}Error: Cargo build failed.${NC}" >&2
        exit 1
    fi
else
    echo -e "${YELLOW}Warning: Cargo.toml not found. Skipping build step.${NC}"
fi

if [[ ! -f "${TARGET_BIN}" ]]; then
    echo -e "${RED}Error: Release binary not found at ${TARGET_BIN}${NC}" >&2
    echo -e "${YELLOW}Note: Run with --download-only to download ROMs before emulator binary is built.${NC}" >&2
    exit 1
fi

# 3. Execution Helper with Timeout Support
run_with_timeout() {
    local timeout_sec="$1"
    local log_file="$2"
    shift 2

    if command -v timeout &>/dev/null; then
        timeout "${timeout_sec}s" "$@" > "${log_file}" 2>&1
        return $?
    elif command -v gtimeout &>/dev/null; then
        gtimeout "${timeout_sec}s" "$@" > "${log_file}" 2>&1
        return $?
    else
        # Portable fallback for macOS without coreutils timeout
        "$@" > "${log_file}" 2>&1 &
        local pid=$!
        local count=0
        while kill -0 $pid 2>/dev/null; do
            if [[ $count -ge $timeout_sec ]]; then
                kill -9 $pid 2>/dev/null || true
                wait $pid 2>/dev/null || true
                echo "Error: Execution timed out after ${timeout_sec}s" >> "${log_file}"
                return 124
            fi
            sleep 1
            ((count++))
        done
        wait $pid
        return $?
    fi
}

# 4. Test Suite Execution Matrix
# Array format: "ROM_PATH|DEFAULT_TIMEOUT_SECONDS|DISPLAY_NAME"
TEST_MATRIX=(
    "${INDIVIDUAL_DIR}/01-special.gb|10|01-special"
    "${INDIVIDUAL_DIR}/02-interrupts.gb|10|02-interrupts"
    "${INDIVIDUAL_DIR}/03-op sp,hl.gb|10|03-op sp,hl"
    "${INDIVIDUAL_DIR}/04-op r,imm.gb|10|04-op r,imm"
    "${INDIVIDUAL_DIR}/05-op rp.gb|10|05-op rp"
    "${INDIVIDUAL_DIR}/06-ld r,r.gb|10|06-ld r,r"
    "${INDIVIDUAL_DIR}/07-jr,jp,call,ret,rst.gb|10|07-jr,jp,call,ret,rst"
    "${INDIVIDUAL_DIR}/08-misc instrs.gb|10|08-misc instrs"
    "${INDIVIDUAL_DIR}/09-op r,r.gb|10|09-op r,r"
    "${INDIVIDUAL_DIR}/10-bit ops.gb|10|10-bit ops"
    "${INDIVIDUAL_DIR}/11-op a,(hl).gb|10|11-op a,(hl)"
    "${ROM_DIR}/instr_timing.gb|10|instr_timing"
    "${ROM_DIR}/cpu_instrs.gb|35|cpu_instrs (Umbrella)"
)

PASSED_COUNT=0
FAILED_COUNT=0
TOTAL_TESTS="${#TEST_MATRIX[@]}"
LOG_TMP_DIR=$(mktemp -d /tmp/gb_test_logs.XXXXXX)

trap 'rm -rf "${LOG_TMP_DIR}"' EXIT

echo -e "\n${BOLD}Step 3: Executing Headless Test Suite (${TOTAL_TESTS} ROMs)${NC}\n"

for entry in "${TEST_MATRIX[@]}"; do
    IFS="|" read -r rom_path default_timeout display_name <<< "${entry}"
    
    timeout_sec="${default_timeout}"
    if [[ -n "${CUSTOM_TIMEOUT}" ]]; then
        timeout_sec="${CUSTOM_TIMEOUT}"
    fi

    log_file="${LOG_TMP_DIR}/${display_name// /_}.log"

    printf "%-35s ... " "${display_name}"
    
    start_time=$(date +%s)
    
    exit_code=0
    run_with_timeout "${timeout_sec}" "${log_file}" "${TARGET_BIN}" --headless "${rom_path}" || exit_code=$?

    end_time=$(date +%s)
    duration=$((end_time - start_time))

    # Evaluate Pass / Fail condition
    if grep -q "Passed" "${log_file}" && ! grep -q "Failed" "${log_file}"; then
        echo -e "${GREEN}[ PASS ]${NC} (${duration}s)"
        ((PASSED_COUNT++))
    else
        if [[ ${exit_code} -eq 124 ]] || grep -q "timed out" "${log_file}"; then
            echo -e "${RED}[ TIMEOUT ]${NC} (${duration}s / limit ${timeout_sec}s)"
        else
            echo -e "${RED}[ FAIL ]${NC} (Exit code: ${exit_code})"
        fi
        if [[ -s "${log_file}" ]]; then
            echo -e "${YELLOW}--- Serial Output Log (${display_name}) ---${NC}"
            cat "${log_file}" | sed 's/^/  /'
            echo -e "${YELLOW}-----------------------------------------${NC}"
        fi
        ((FAILED_COUNT++))
    fi
done

# 5. Summary Output
echo -e "\n${BOLD}${BLUE}====================================================${NC}"
echo -e "${BOLD}Test Results Summary:${NC}"
echo -e "  Total Tests Run : ${TOTAL_TESTS}"
echo -e "  Passed          : ${GREEN}${PASSED_COUNT}${NC} / ${TOTAL_TESTS}"
echo -e "  Failed          : ${RED}${FAILED_COUNT}${NC} / ${TOTAL_TESTS}"
echo -e "${BOLD}${BLUE}====================================================${NC}"

if [[ "${FAILED_COUNT}" -eq 0 ]]; then
    echo -e "${BOLD}${GREEN}SUCCESS: All ${TOTAL_TESTS} Blargg Game Boy test ROMs PASSED!${NC}\n"
    exit 0
else
    echo -e "${BOLD}${RED}FAILURE: ${FAILED_COUNT} / ${TOTAL_TESTS} test(s) failed.${NC}\n"
    exit 1
fi
