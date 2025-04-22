#!/bin/bash
SCRIPT_ROOT=$(cd $(dirname $0);pwd)

pushd $SCRIPT_ROOT
    # VERSION=$(cargo metadata --format-version=1 --no-deps | jq  '.packages[] | select(.name == "peercast-root") | .version')
    # latest(tag)はローカルビルドで簡単にバージョンを指定する為に使うに留め
    # docker hubにアップロードすることはないように注意する
    docker build -t peercast-root:latest -f ./docker/Dockerfile.peercast-root ./
popd
