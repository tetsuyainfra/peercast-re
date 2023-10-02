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
#PEERCAST_RT_BUILD_NPM_REBUILD=true cross build --profile release

