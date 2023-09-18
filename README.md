# PeerCast Re:

PeerCast Re: is implemented in Rust-Lang.
PeerCast is P2P based livestreaming software.

Generally write it as peercast-re.

# 開発メモ
```
# 1. Prepare
# Install Docker, jq, cargo, etc...

# 2. generate Api Package from OpenApi
./api.codegen.sh

# 3. Build
PEERCAST_RT_BUILD_NPM_REBUILD=true cargo build

# 4. run
cargo run
```

## UI開発時のビルド
```
# on main window
PEERCAST_RT_FRONTEND_UI_MODE=proxy cargo run

# on other window
cd client
npm run dev
```


## Cross Compile(on Linux build Win-Binary)

Dockerは必須

```
cargo install cross

cp Cross.toml{.sample,}

cross build
ls target/x86_64-pc-windows-gnu/debug

# release build
cross build --relase profile
ls target/x86_64-pc-windows-gnu/release
```

### Api

| name                 |          URL           |
| :------------------- | :--------------------: |
| Swagger Editor       | http://localhost:8001/ |
| Swagger UI           | http://localhost:8002/ |
| ~~Swagger API mock~~ | http://localhost:8003/ |


