#!/bin/bash
SCRIPT_ROOT=$(cd $(dirname $0);pwd)

TARGET=${TARGET:-x86_64-unknown-linux-gnu}
PROFILE=${PROFILE:-dev}

if [ "$PROFILE" = "release" ]; then
    PROFILE_DIR="release"
else
    PROFILE_DIR="debug"
fi

pushd $SCRIPT_ROOT
    # VERSION=$(cargo metadata --format-version=1 --no-deps | jq  '.packages[] | select(.name == "peercast-root") | .version')
    # latest(tag)はローカルビルドで簡単にバージョンを指定する為に使うに留め
    # docker hubにアップロードすることはないように注意する
    docker build -t peercast-root:latest -f ./docker/Dockerfile.peercast-root \
      --build-arg TARGET=${TARGET} \
      --build-arg PROFILE=${PROFILE_DIR} \
      ./
popd
