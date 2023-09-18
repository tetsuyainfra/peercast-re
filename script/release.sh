#!/bin/bash
set -ex

PEERCAST_RT_BUILD_NPM_REBUILD=true cargo build --profile release
PEERCAST_RT_BUILD_NPM_REBUILD=true cross build --profile release



ls -lh target/release
