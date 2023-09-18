#!/bin/bash
set -e

SCRIPT_DIR=$(cd $(dirname $0); pwd)
PROJECT_DIR=$(cd $SCRIPT_DIR; cd ../ ; pwd)
PACKAGE_VERSION=$(cargo metadata --no-deps --format-version=1 | jq --raw-output .packages[0].version)

echo SCRIPT_DIR=$SCRIPT_DIR
echo PROJECT_DIR=$PROJECT_DIR
echo PACKAGE_VERSION=$PACKAGE_VERSION

set -ex
cd $PROJECT_DIR

PEERCAST_RT_BUILD_NPM_REBUILD=true cargo build --profile release
PEERCAST_RT_BUILD_NPM_REBUILD=true cross build --profile release

rm -f ./publish/*

pushd target/release
    cp -a ${PROJECT_DIR}/HOW_TO_USE.md ./
    tar zcvf ${PROJECT_DIR}/publish/peercast-re_v${PACKAGE_VERSION}_x64.tar.gz peercast-re HOW_TO_USE.md
popd

pushd target/x86_64-pc-windows-gnu/release
    cp -a ${PROJECT_DIR}/HOW_TO_USE.md ./
    zip      ${PROJECT_DIR}/publish/peercast-re_v${PACKAGE_VERSION}_x64.zip peercast-re.exe HOW_TO_USE.md
popd

ls -lh target/release
ls -lh target/x86_64-pc-windows-gnu/release
ls -lh publish/
