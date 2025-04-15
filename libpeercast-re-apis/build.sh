#!/bin/bash

set -ex
SCRIPT_ROOT=$(cd $(dirname $0);pwd)

pushd $SCRIPT_ROOT
    UID=${UID} GID=${GID} docker compose -f docker-compose.build-api.yml  --profile api up --remove-orphans
popd

