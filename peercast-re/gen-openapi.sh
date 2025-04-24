#!/bin/bash
SCRIPT_ROOT=$(cd $(dirname $0);pwd)

PROFILE=${PROFILE:-dev}

set -ex

pushd $SCRIPT_ROOT
    cargo run --bin gen-openapi > openapi.json
popd
