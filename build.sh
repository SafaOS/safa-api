#!/bin/bash
# Builds the safa-api library and the crt0 object
# output is in the `out` directory

set -euo pipefail

libsafa_api=$(cargo build --release --message-format=json-render-diagnostics | jq -r 'select(.reason == "compiler-artifact" and (.target.kind | index("staticlib"))) | .filenames[] | select(endswith(".a"))')
mkdir -p out

cd extra
cargo rustc --lib --release -- --emit obj=../out/crt0.o
cp $libsafa_api ../out/libsafa_api.a

