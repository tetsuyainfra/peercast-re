#/bin/bash
set -ex


docker run --rm --init -v $PWD:/home/marp/app -e LANG=$LANG -p 8080:8080 -p 37717:37717 \
    -e MARP_USER="$(id -u):$(id -g)" \
    marpteam/marp-cli -s .
