#!/bin/bash
SCRIPT_ROOT=$(cd $(dirname $0);pwd)

PROFILE=${PROFILE:-dev}

set -ex

pushd $SCRIPT_ROOT
    pushd ./client
        pnpm install --frozen-lockfile
        pnpm run clean
        pnpm run gen-api
        pnpm run gen-api2
        pnpm run build
    popd
popd
