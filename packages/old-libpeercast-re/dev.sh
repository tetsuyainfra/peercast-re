#!/bin/bash

set -ex

./api.codegen.sh

PEERCAST_RT_FRONTEND_UI_MODE=proxy cargo run
