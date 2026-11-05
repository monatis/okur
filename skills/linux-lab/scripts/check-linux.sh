#!/usr/bin/env bash
# The quality gate's checks for what only Linux builds (the pre-commit gate runs on Windows,
# where okur-linux is empty): clippy with the gate's lints, then the tests of the Linux crates.
# Run in the lab, from anywhere:
#   bash /mnt/d/okur-linux/check-linux.sh
# Prints LINUX CHECK PASS or LINUX CHECK FAIL as its last line.
set -uo pipefail
cd ~/okur
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-4}" RUST_TEST_THREADS="${RUST_TEST_THREADS:-4}"
crates=(-p okur-linux -p okur-linux-model -p xa11y-linux)
if ! cargo clippy -q "${crates[@]}" --all-targets -- -D warnings \
    -D clippy::indexing_slicing -D clippy::string_slice; then
    echo "LINUX CHECK FAIL: clippy"
    exit 1
fi
tests=$(cargo test -q "${crates[@]}" 2>&1)
status=$?
echo "$tests" | grep -E "^test result|FAILED|panicked|^error" 
if [ $status -ne 0 ]; then
    echo "LINUX CHECK FAIL: tests"
    exit 1
fi
echo "LINUX CHECK PASS"
