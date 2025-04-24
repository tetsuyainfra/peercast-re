#!/bin/bash
SCRIPT_ROOT=$(cd $(dirname $0);pwd)

PROFILE=${PROFILE:-dev}
TARGET=${TARGET:-x86_64-unknown-linux-gnu}

set -ex

pushd $SCRIPT_ROOT
    PROFILE=${PROFILE} ./peercast-re/build.sh

    cargo build --profile=${PROFILE} --target=${TARGET}
popd
