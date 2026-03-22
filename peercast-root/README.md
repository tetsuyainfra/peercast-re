# peercast-root
Peercastの配信告知用サーバーです。最低限のRootモードPeercastとindex.txtを配信できるHTTPサーバーを実装しています。


# 制限について
- 現在のpeercast-rootでは一つのPeerCast-Tracker(配信者のピアキャスト)から一つのチャンネル情報しか送信できません。
  - ずっと仕様を勘違いしていました。将来のアップデートでなんとかします

# GenreによるYP掲載の制御
- ジャンル文字列の頭に[yp]*1の文字を含めてください。
- '?'を[yp]の次に含めると、リスナー数を非表示にできます。 *2
  - 例：yp?Game
- 名前空間は[yp]の後に文字列を指定し、コロンで終えて下さい。*3
  - 例：ypXYZ?Game
- @の数によって視聴者への表示制限を行えます *4
  - @ ポートチェックを行います。
  - @@ ポートチェックと帯域チェック（配信ビットレート基準）を行います。
  - @@@ ポートチェックと帯域チェック（2Mbps制限 *5）を行います。（光固め等に使用）

## 起動オプションによる変更
- *1 : [yp]は```--yp-name=希望の文字列```オプションで変更することができます
- *2 名前空間の使用可否は```--yp-neme-spaceable=false```でオフにできます
- *3 リスナー数非表示は```--yp-listener-hideable=false```でオフにできます
- *4 YPで使える最大の制限を```--yp_restrict_port_level=[none|port-check|...]```でレベルを変更できます。
  - 他の設定値は```--help```を参照してください。
  - YPの設定が[port-check]の時に配信者がジャンルに@を2個以上指定してもport-check以上は利用できません。
- *5 制限帯域は```--yp-limit-speed=2000```で変更できます

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

## Appendix Binary
- create_info : index.txtに追加できるFooterテキストを定義するTOMLを出力するためのコマンドです
- peercast-db-cli : peercast-rootが作成するDBを読み書きできるCLIです

## TODO
- PCPで受け取ったデータでチャンネル情報を更新する
- 5分後にチャンネルを削除する機能をつける
- Dockerを作る
- Debian Pacakgeを生成する
- IPv4 in IPv6で最適化(DB上も16bytes固定になっていいかも？)
- minijinjaよりhandlebarsの方が標準的でよさそう

### MEMO
