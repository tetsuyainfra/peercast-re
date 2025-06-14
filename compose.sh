#!/bin/bash
SCRIPT_ROOT=$(cd $(dirname $0);pwd)
set -ex



pushd $SCRIPT_ROOT
    docker compose $@
popd
