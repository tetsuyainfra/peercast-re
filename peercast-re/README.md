# PeerCast-Re





## TODO
- Socketの管理はsystemdにやらせたい
  - https://tm23forest.com/contents/systemd-socket-passing
  - listenfd crateを使えばよさそう
  - dev時は systemfd を使えばsystemdの代わりにsocket作ってくれる
