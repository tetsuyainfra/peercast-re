#!/bin/bash

set -ex

docker-compose -f ./docker-compose.openapi.yml "$@"
