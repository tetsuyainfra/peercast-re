# PeerCast Re: の使い方

## 注意
- 現状、RTMPをPeerCast-Reに送信して、ブラウザから受信したデータをFLVストリームとして表示することしかできません。
- コンフィグファイルが自動で作成されます。（中身は空です）場所については下の項目を確認してください。


## お試しの方法
プログラム(peercast.exe)を起動するとコマンドプロンプト(黒い画面)が現れます。
その中に次のログ(記録)が表示されます。

```
2023-09-18T07:44:47.436683Z  INFO src/app/cui.rs:182: UI          -> http://localhost:17144/ui/
2023-09-18T07:44:47.437974Z  INFO src/app/cui.rs:187: rtmp server -> rtmp://localhost:11935
```

この```http://localhost:17144/ui/```(※1)がブラウザからアクセスできるURLになります。
またOBSに指定するRTMPサーバーアドレスが```rtmp://localhost:11935```です。
※ 正確にはappkeyが必要なので ```rtmp://localhost:11935/req1```(※2) になります

1. ブラウザで(※1)を開きます
2. 左側のメニューでChannelsをクリックするとチャンネル一覧のページに飛びます
3. Openをクリックすると再生画面が開きます
4. OBSで(※2)をサーバーアドレスに設定して配信開始してください
5. ブラウザ上で放送が確認できます

※ ちなみにConfigで現在の設定情報を確認できますが、設定変更はできません。


## 保存されるコンフィグファイルの場所
- Linux: $HOME/.config/peercast/peercast-re.ini
- Windows: %USERPROFILE%\AppData\Roaming\peercast\peercast-re.ini
