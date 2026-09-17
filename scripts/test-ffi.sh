#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."
libdir="${1:-target/dev/debug}"
mkdir -p artifacts/tests
cc -std=c11 -Wall -Wextra -Werror -pthread tests/ffi_client.c -L"$libdir" -llicense_guard -Wl,-rpath,"$PWD/$libdir" -o artifacts/tests/ffi-client
c++ -x c++ -std=c++17 -Wall -Wextra -Werror -fsyntax-only tests/ffi_client.c
artifacts/tests/ffi-client
