# PeerCastStation コードリーディングメモ
LICENCE：GPLv3 (PeCaStがそうだからコードが含まれるこのメモもそれに従います)

# 下流から上流へのAtomの処理の流れ
OutputStraemへのHTTP接続とPCP接続へのアップグレード
ここでサービスにアタッチしてる
```csharp
    public static void BuildApp(IAppBuilder builder)
    {
      builder.MapGET("/channel", sub => {
        sub.Run(PCPRelayHandler.Invoke);
      });
    }
```
1. PCPRelayHandler.Invoke() アップグレードしてハンドラー君に処理を開始させている
```csharp 
ctx.Upgrade(async opaqueEnv => {
  var ct = (CancellationToken)opaqueEnv[OwinEnvironment.Opaque.CallCancelled];
  var stream = (Stream)opaqueEnv[OwinEnvironment.Opaque.Stream];
  stream.ReadTimeout = Timeout.Infinite;
  var handler = new PCPRelayHandler(channel, logger);
  await handler.ProcessStream(stream, remoteEndPoint, requestPos, ct).ConfigureAwait(false);
});
```
2. PCPRelayHandler.ProcessStream()
3. PCPRelayHandler.ReadAndProcessAtom()
4. PCPRelayHandler.ProcessAtom()
ここで到着したAtomを処理する・・・
```csharp
             if (atom.Name==Atom.PCP_HELO)       await OnPCPHelo(stream, sink, atom, cancel_token).ConfigureAwait(false);
        else if (atom.Name==Atom.PCP_OLEH)       await OnPCPOleh(stream, sink, atom, cancel_token).ConfigureAwait(false);
        else if (atom.Name==Atom.PCP_OK)         await OnPCPOk(stream, sink, atom, cancel_token).ConfigureAwait(false);
        else if (atom.Name==Atom.PCP_CHAN)       await OnPCPChan(stream, sink, atom, cancel_token).ConfigureAwait(false);
        else if (atom.Name==Atom.PCP_CHAN_PKT)   await OnPCPChanPkt(stream, sink, atom, cancel_token).ConfigureAwait(false);
        else if (atom.Name==Atom.PCP_CHAN_INFO)  await OnPCPChanInfo(stream, sink, atom, cancel_token).ConfigureAwait(false);
        else if (atom.Name==Atom.PCP_CHAN_TRACK) await OnPCPChanTrack(stream, sink, atom, cancel_token).ConfigureAwait(false);
        else if (atom.Name==Atom.PCP_BCST)       await OnPCPBcst(stream, sink, atom, cancel_token).ConfigureAwait(false);
        else if (atom.Name==Atom.PCP_HOST)       await OnPCPHost(stream, sink, atom, cancel_token).ConfigureAwait(false);
        else if (atom.Name==Atom.PCP_QUIT)       await OnPCPQuit(stream, sink, atom, cancel_token).ConfigureAwait(false);
```
でもやることないよな？っておもったらその先のメソッドで
```csharp
     private Task OnPCPOk(Stream stream, ChannelSink sink, Atom atom, CancellationToken cancel_token)
     {
       return Task.Delay(0);
     }
     private Task OnPCPChan(Stream stream, ChannelSink sink, Atom atom, CancellationToken cancel_token)
     {
       return Task.Delay(0);
     }...
```
意味があるのが次の3つ
- OnPCPBcst
- OnPCPHost
- OnPCPQuit(今回は省略)
```csharp
      private async Task OnPCPBcst(Stream stream, ChannelSink sink, Atom atom, CancellationToken cancel_token)
      {
        if (atom.Children==null) {
          throw new InvalidDataException($"{atom.Name} has no children.");
        }
        var dest = atom.Children.GetBcstDest();
        var ttl = atom.Children.GetBcstTTL();
        var hops = atom.Children.GetBcstHops();
        var from = atom.Children.GetBcstFrom();
        var group = atom.Children.GetBcstGroup();
        if (ttl != null &&
            hops != null &&
            group != null &&
            from != null &&
            dest != peerCast.SessionID &&
            ttl>0) {
          logger.Debug("Relaying BCST TTL: {0}, Hops: {1}", ttl, hops);
          var bcst = new AtomCollection(atom.Children);
          bcst.SetBcstTTL((byte)(ttl - 1));
          bcst.SetBcstHops((byte)(hops + 1));
          channel.Broadcast(sink.Peer.Host, new Atom(atom.Name, bcst), group.Value);
        }
        if (dest==null || dest==peerCast.SessionID) {
          logger.Debug("Processing BCST({0})", dest==null ? "(null)" : dest.Value.ToString("N"));
          foreach (var c in atom.Children) {
            await ProcessAtom(stream, sink, c, cancel_token).ConfigureAwait(false);
          }
        }
      }
```
なるほど下流からデータが着たらChannel.Braodcastにぶちこんで上流に送ってるっぽい
- destが自分のID or Nullなら自分で内容を処理するために再度ProcessAtomにぶちこむ（もちろん上流には送らない）
  - このAtomのChildrenにHostが含まれててそれを処理しているのではないかな？(予想)

```csharp
      private Task OnPCPHost(Stream stream, ChannelSink sink, Atom atom, CancellationToken cancel_token)
      {
        if (atom.Children==null) {
          throw new InvalidDataException($"{atom.Name} has no children.");
        }
        var session_id = atom.Children.GetHostSessionID();
        if (session_id!=null) {
          var node = channel.Nodes.FirstOrDefault(x => x.SessionID.Equals(session_id));
          var host = new HostBuilder(node);
          if (node==null) {
            host.SessionID = (Guid)session_id;
          }
          host.Extra.Update(atom.Children);
          host.DirectCount = atom.Children.GetHostNumListeners() ?? 0;
          host.RelayCount = atom.Children.GetHostNumRelays() ?? 0;
          var flags1 = atom.Children.GetHostFlags1();
          if (flags1 != null) {
            host.IsFirewalled  = (flags1.Value & PCPHostFlags1.Firewalled) != 0;
            host.IsTracker     = (flags1.Value & PCPHostFlags1.Tracker) != 0;
            host.IsRelayFull   = (flags1.Value & PCPHostFlags1.Relay) == 0;
            host.IsDirectFull  = (flags1.Value & PCPHostFlags1.Direct) == 0;
            host.IsReceiving   = (flags1.Value & PCPHostFlags1.Receiving) != 0;
            host.IsControlFull = (flags1.Value & PCPHostFlags1.ControlIn) == 0;
          }

          var endpoints = atom.Children.GetHostEndPoints();
          if (endpoints.Length>0 && (host.GlobalEndPoint==null || !host.GlobalEndPoint.Equals(endpoints[0]))) {
            host.GlobalEndPoint = endpoints[0];
          }
          if (endpoints.Length>1 && (host.LocalEndPoint==null || !host.LocalEndPoint.Equals(endpoints[1]))) {
            host.LocalEndPoint = endpoints[1];
          }
          logger.Debug($"Updating Node: {host.GlobalEndPoint}/{host.LocalEndPoint}({host.SessionID.ToString("N")})");
          channel.AddNode(host.ToHost());
          if (sink.Peer.Host.SessionID==host.SessionID) {
            sink.Peer = new PeerInfo(host.ToHost(), sink.Peer.UserAgent, sink.Peer.RemoteEndPoint);
          }
        }
        return Task.Delay(0);
      }
```
なるほど下流からHostデータが着たらHostデータをまとめてchannelにAddNodeしている
っていうかHOSTがAtomのストリームにいきなり現れるのだろうか(↑のBCSTの子に含まれていたAtomかな？)



# Channel内部でのブロードキャストの方法
各チャンネルに送られてきたAtomをDoProcessで処理したらChannel.Broadcast() メソッドを呼び出している
内部でSourceStream.PostとOutStream.OnBroadcastに(必要であれば両方へ)メッセージを送っている

1. SouceStream.Post -> SourceConnectionBase.Post -> public class PCPSourceConnection.DoPostにたどり着く
- uphostからのデータははじいている　= Root(YP)からパケットが来るわけないから
- BlockingCollection<Atom> postedAtoms してキューに追加しているっぽい
- postedAtomsはProcessPostで解放っていうか処理されてるっぽい
  - これは結局Rootへ送っているってことか

1. OutStream.OnBroadcast ->


# ChannelID
- PeCaStではチャンネル情報からGnuIDを作っている？(未確認)
  - 同じ値で配信するとチャンネルIDが被る
  - だから何か有ってPeCaStクラッシュした後、YPに乗らない問題が起きるのかな？
- どうなっているのがいいのか。。。
-
