#!/bin/bash

set -ex
SCRIPT_ROOT=$(cd $(dirname $0);pwd)
export USER_ID=$(id -u)
export GROUP_ID=$(id -g)

pushd $SCRIPT_ROOT
    docker compose -f docker-compose.build-api.yml  --profile api up --remove-orphans
    ls -l gen
popd

