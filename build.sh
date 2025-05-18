#!/bin/bash
SCRIPT_ROOT=$(cd $(dirname $0);pwd)

TARGET=${TARGET:-x86_64-unknown-linux-gnu}
PROFILE=${PROFILE:-dev}

set -ex

pushd $SCRIPT_ROOT
    PROFILE=${PROFILE} ./peercast-re/build.sh

    cargo build --profile=${PROFILE} --target=${TARGET}
popd
