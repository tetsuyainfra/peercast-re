#!/bin/bash

set -ex
SCRIPT_ROOT=$(cd $(dirname $0);pwd)
USER_ID=$(id -u)
GROUP_ID=$(id -g)

pushd $SCRIPT_ROOT
    id
    docker compose -f docker-compose.build-api.yml  --profile api up --remove-orphans
    ls -l gen
popd

