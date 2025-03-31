#/bin/bash
set -ex

docker run --rm --init -v $PWD:/home/marp/app/ -e LANG=$LANG  -e MARP_USER="$(id -u):$(id -g)" marpteam/marp-cli \
    2023_09_12_cascket/slide.md --pdf


docker run --rm --init -v $PWD:/home/marp/app -e LANG=$LANG -p 8080:8080 -p 37717:37717 marpteam/marp-cli -s .
