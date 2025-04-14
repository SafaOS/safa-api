#!/bin/bash
# Builds the safa-api library and the crt0 object
# output is in the `out` directory

set -euo pipefail

libsafa_api=$(cargo rustc --crate-type=staticlib --release --message-format=json-render-diagnostics | jq -r 'select(.reason == "compiler-artifact" and (.target.kind | index("staticlib"))) | .filenames[] | select(endswith(".a"))')

mkdir -p out
cp $libsafa_api out/libsafa_api.a
