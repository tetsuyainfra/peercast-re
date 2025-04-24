#!/bin/bash
SCRIPT_ROOT=$(cd $(dirname $0);pwd)

PROFILE=${PROFILE:-dev}

set -ex

pushd $SCRIPT_ROOT
    pushd ./client
        npm run clean
        npm run gen-api
        npm run gen-api2
        npm run build
    popd
popd
