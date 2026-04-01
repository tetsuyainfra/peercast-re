

- YP(Http Server)
- Browser
- PeerCast(Root) : YelloPage
- PeerCast(Tracker) : 配信者
- PeerCast(Relay) : 視聴者

ちなみにビットトレントの場合と違うので混同注意
- Tracker: Seed,Peerの接続情報を配布するサーバー
- Seed: 配布者
- Peer: 接続/DLしてる人


- YPはTrackerのIPも保持しているので/stream/{channel_id}で問い合わせされたら
  503を返して、PCPで次に接続するべきホストを返す
  200を返す場合はTrackerがYPになってる場合（そんなことあるの？）
