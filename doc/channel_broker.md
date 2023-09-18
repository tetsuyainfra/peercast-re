

# ChannelBrokerの仕事

- Channelが作成されたらChannelBrokerが作成され、Channelが消えるまで存続する
- ChannelBroker内部ではスレッドが起動されて、シングルタスクでデータが処理される
- ChannelBrokerに接続するには接続IDとデータの返却先を渡す
  - ConnectionId, mpsc::UnboundSender<ChannelMessage>
- ChannelBroker内部でRtmpEvent -> Atomへの変換が行われる事が望ましいのでは？
  - そうすればrecieverはAtomMessageを受け取る事だけ考えれば良い
