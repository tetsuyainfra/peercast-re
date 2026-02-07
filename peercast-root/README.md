# peercast-root
Peercastの配信告知用サーバーです。最低限のRootモードPeercastとindex.txtを配信できるHTTPサーバーを実装しています。

## 開発について
```
sqlx database setup
cargo run

# run peercast-db-cli
cargo run-db
```


### SQLXについて
環境変数をmise.tomlで制御しています。現在のディレクトリによってSQLXのSQL文静的解析の方法が変わります。
- SQLX_OFFLINE=false : DATABASE_URLを参照して解析
- SQLX_OFFLINE=true  : .sqlx/内のJSONを参照して解析

- peercast-re/ 
  - SQLX_OFFLINE=true
- peercast-re/peercast-root/(現在のフォルダ)
  - SQLX_OFFLINE=false
  - DATABASE_URL=${PWD}/temp/peercast-root.db

### DBのschemaを更新した時
```
cargo sqlx prepare
```


## TODO
- IPv4 in IPv6で最適化(DB上も16bytes固定になっていいかも？)

## Appendix Binary
- create_info : index.txtに追加できるFooterテキストを定義するTOMLを出力するためのコマンドです
- peercast-db-cli : peercast-rootが作成するDBを読み書きできるCLIです
