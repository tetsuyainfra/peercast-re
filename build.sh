#!/bin/bash

set -ex

docker-compose -f docker-compose.build-api.yml  --profile api up --remove-orphans

PEERCAST_RT_BUILD_NPM_REBUILD=true cargo build
