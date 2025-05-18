#!/bin/bash
SCRIPT_ROOT=$(cd $(dirname $0);pwd)
set -ex

pushd $SCRIPT_ROOT
    PROFILE=dev ./build.sh
    PROFILE=dev ./build.docker.sh
popd
