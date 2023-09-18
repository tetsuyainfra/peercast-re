#!/bin/bash
# usage : ./api.codegen.sh
#       : GEN_DOC=1 ./api.codegen.sh

set -ex

if !(type "jq" > /dev/null 2>&1); then
    echo "Please install jq command"
    exit -1
fi

PACKAGE_VERSION=$(cargo metadata --no-deps --format-version=1 | jq --raw-output .packages[0].version)

# docker run --rm --user ${PEERCAST_RT_DOCKER_USER_ID}:${PEERCAST_RT_DOCKER_GROUP_ID} -v "${PWD}:/local" openapitools/openapi-generator-cli list

if [ -n "$GEN_DOC" ]; then
    docker run --rm --user ${PEERCAST_RT_DOCKER_USER_ID}:${PEERCAST_RT_DOCKER_GROUP_ID} -v "${PWD}:/local" openapitools/openapi-generator-cli \
      help generate > gen/help.generate.txt
    docker run --rm --user ${PEERCAST_RT_DOCKER_USER_ID}:${PEERCAST_RT_DOCKER_GROUP_ID} -v "${PWD}:/local" openapitools/openapi-generator-cli \
        config-help  -g rust > gen/config-help.rust.txt
    docker run --rm --user ${PEERCAST_RT_DOCKER_USER_ID}:${PEERCAST_RT_DOCKER_GROUP_ID} -v "${PWD}:/local" openapitools/openapi-generator-cli \
        config-help  -g rust-server > gen/config-help.rust.txt
    docker run --rm --user ${PEERCAST_RT_DOCKER_USER_ID}:${PEERCAST_RT_DOCKER_GROUP_ID} -v "${PWD}:/local" openapitools/openapi-generator-cli \
        config-help  -g typescript-fetch > gen/config-help.ts-fetch.txt
fi

# peercast-rtで使うmodelはこっちから引っ張ってくる
rm -rf ./gen/rust
docker run --rm --user ${PEERCAST_RT_DOCKER_USER_ID}:${PEERCAST_RT_DOCKER_GROUP_ID} -v "${PWD}:/local" openapitools/openapi-generator-cli generate \
    -i /local/api/openapi.yaml \
    -g rust \
    -o /local/gen/rust \
    -p packageVersion=${PACKAGE_VERSION} \
    -c /local/api/rust-config.yaml

# mockを作るときはこっち
rm -rf ./gen/rust-server
docker run --rm --user ${PEERCAST_RT_DOCKER_USER_ID}:${PEERCAST_RT_DOCKER_GROUP_ID} -v "${PWD}:/local" openapitools/openapi-generator-cli generate \
    -i /local/api/openapi.yaml \
    -g rust-server \
    -o /local/gen/rust-server \
    -p packageName=peercast-rt-api-server \
    -p packageVersion=${PACKAGE_VERSION}

# rm -rf ./gen/js
# docker run --rm --user ${PEERCAST_RT_DOCKER_USER_ID}:${PEERCAST_RT_DOCKER_GROUP_ID} -v "${PWD}:/local" openapitools/openapi-generator-cli generate \
#     -i /local/api/openapi.yaml \
#     -g javascript \
#     -o /local/gen/js \
#     -p packageVersion=${PACKAGE_VERSION}

rm -rf ./gen/ts-fetch
docker run --rm --user ${PEERCAST_RT_DOCKER_USER_ID}:${PEERCAST_RT_DOCKER_GROUP_ID} -v "${PWD}:/local" openapitools/openapi-generator-cli generate \
    -i /local/api/openapi.yaml \
    -g typescript-fetch \
    -o /local/gen/ts-fetch \
    -p packageVersion=${PACKAGE_VERSION} \

