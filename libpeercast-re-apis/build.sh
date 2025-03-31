#!/bin/bash

set -ex

docker-compose -f docker-compose.build-api.yml  --profile api up --remove-orphans

