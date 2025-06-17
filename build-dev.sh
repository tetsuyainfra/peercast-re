#!/bin/bash
SCRIPT_ROOT=$(cd $(dirname $0);pwd)
set -ex

TARGET_BIN=${1:-ALL}
# 大文字に正規化
if [ "ALL" = "${TARGET_BIN^^}" ] ; then
    TARGET_BIN=ALL
fi

pushd $SCRIPT_ROOT
    PROFILE=dev ./_build.sh $TARGET_BIN
    PROFILE=dev ./_build.docker.sh $TARGET_BIN
popd
