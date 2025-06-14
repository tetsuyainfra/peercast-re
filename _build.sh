#!/bin/bash
SCRIPT_ROOT=$(cd $(dirname $0);pwd)

TARGET=${TARGET:-x86_64-unknown-linux-gnu}
PROFILE=${PROFILE:-dev}


TARGET_BIN=${1:-ALL}
# 大文字に正規化
if [ "ALL" = "${TARGET_BIN^^}" ] ; then
    TARGET_BIN=ALL
fi
echo "TARGET_BIN: $TARGET_BIN"
echo "TARGET    : $TARGET"
echo "PROFILE   : $PROFILE"

set -ex

pushd $SCRIPT_ROOT
    if [ "peercast-re" = "$TARGET_BIN" -o "ALL" = "$TARGET_BIN" ] ; then
        PROFILE=${PROFILE} ./peercast-re/build.sh
    fi

    if [ "peercast-root" = "$TARGET_BIN" -o "ALL" = "$TARGET_BIN" ] ; then
        : # PROFILE=${PROFILE} ./peercast-root/build.sh
    fi

    if [ "ALL" == "$TARGET_BIN" ]; then
        cargo build --profile=${PROFILE} --target=${TARGET}
    else
        cargo build --profile=${PROFILE} --target=${TARGET} --bin $TARGET_BIN
    fi
popd