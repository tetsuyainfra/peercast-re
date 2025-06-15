#!/bin/bash
SCRIPT_ROOT=$(cd $(dirname $0);pwd)

TARGET=${TARGET:-x86_64-unknown-linux-gnu}
PROFILE=${PROFILE:-dev}

if [ "$PROFILE" = "release" ]; then
    PROFILE_DIR="release"
    RUST_LOG=${RUST_LOG:-info}
else
    PROFILE_DIR="debug"
    RUST_LOG=${RUST_LOG:-debug}
fi


TARGET_IMAGE=${1:-ALL}
# 大文字に正規化
if [ "ALL" = "${TARGET_IMAGE^^}" ] ; then
    TARGET_IMAGE=ALL
fi

echo "TARGET_IMAGE: $TARGET_IMAGE"
echo "TARGET      : $TARGET"
echo "PROFILE     : $PROFILE"
echo "RUST_LOG    : $RUST_LOG"


pushd $SCRIPT_ROOT
    # VERSION=$(cargo metadata --format-version=1 --no-deps | jq  '.packages[] | select(.name == "peercast-root") | .version')
    # latest(tag)はローカルビルドで簡単にバージョンを指定する為に使うに留め
    # docker hubにアップロードすることはないように注意する

    if [ "peercast-re" = "$TARGET_IMAGE" -o "ALL" = "$TARGET_IMAGE" ] ; then
        :
    fi

    if [ "peercast-root" = "$TARGET_IMAGE" -o "ALL" = "$TARGET_IMAGE" ] ; then
        docker build -t peercast-root:latest -f ./docker/Dockerfile.peercast-root \
          --build-arg TARGET=${TARGET} \
          --build-arg PROFILE=${PROFILE_DIR} \
          --build-arg RUST_LOG=${RUST_LOG} \
          ./
    fi

popd
