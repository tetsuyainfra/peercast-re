# リファクタリング
- コードのお引越し
  - uiはとりあえず無理やり動かしたけどー
- src/pcp/decode, src/pcp/builderの統合
  - atomの作成、変更を統合して操作できるようにしたい
  - XXXInfoからAtomの作成
  - PcpXXXを操作してAtomの書き換え
- OpenAPIのコード生成をaxum+utopia, typescript+@hey-api/openapi-tsに移行する
  - dockerでのコード生成が面倒なので

# Debian Script Debug
- sudo dpkg -D2 -i target/debian/peercast-re_0.1.0-1_amd64.deb
  - dpkg -Dhで詳細
