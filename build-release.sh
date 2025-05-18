#!/bin/bash
SCRIPT_ROOT=$(cd $(dirname $0);pwd)
set -ex

pushd $SCRIPT_ROOT
    PROFILE=release ./build.sh
    PROFILE=release ./build.docker.sh
popd
