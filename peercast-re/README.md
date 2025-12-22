# PeerCast-Re





## TODO
- Socketの管理はsystemdにやらせたい
  - https://tm23forest.com/contents/systemd-socket-passing
  - listenfd crateを使えばよさそう
  - dev時は systemfd を使えばsystemdの代わりにsocket作ってくれる


## MEMORY
- tracing-logはtracing-subscriberのfeaturesでtracing-logを指定していれば必要ない（自動で有効にしてくれる）
