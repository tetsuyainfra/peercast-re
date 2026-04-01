
# TODO
- /ui以下でfallbackしてindex.html読み込ませる必要あり
- rust-serverのほうが吐き出すModelが綺麗
- 

# いろいろ
# PeerCastStation と PeerCast-YTの違いっぽいの
- PCP_HOST_OLDPOS 等のHOSTがどこまで視聴したかの情報を使ったり使わなかったり
  - つまるところどっちも下流からきたパケットはそのまま上流に流してるのか


# RTMP to FLV
- https://www.slideshare.net/kumaryu/2-17056211
  - ありがてぇありがてぇ
- http://download.macromedia.com/f4v/video_file_format_spec_v10_1.pdf

## PCPがIncommingされた後のMainLoop
- https://github.com/plonk/peercast-yt/blob/787be6405cc2d82a5d26c0023aaa5d1973c13802/core/common/servent.cpp#L1381
  - updateInfoの中で30秒毎にPCP_BCST ATOMを送信しているっぽい

## ffmpeg で testsrc

https://nico-lab.net/testsrc_with_ffmpeg/

ffmpeg \
 -f lavfi -i testsrc2=1280x720:r=30:d=30 \
 -f lavfi -i sine=frequency=220:beep_factor=4:duration=30 \
 ./tmp/output.mp4

## ffmpeg で rtmp send

ffmpeg -re -stream_loop -1 -i ./tmp/output.mp4 -vcodec libx264 -vprofile baseline -g 30 -acodec aac -strict -2 -f flv rtmp://localhost:11935

## 開発中の Frontend の扱い

-> Production は static だけでいいよね
-> Development は static or development
-> PEERCAST_RT_FRONTEND_UI_MODE=static|dynami
-> で切り替えられるのが楽でよさそう
こんな感じでいいのでは？

## タスクランナーを自前で用意するのはどうか

- cargo run / npm run dev を実行して終了通知おくるだけで良いし・・・・

```
$ cargo install cargo-make

# to development
$ cargo make dev

# to release
```

### OnPCPBroadCast

https://github.com/kumaryu/peercaststation/blob/6184647e600ec3a388462169ab7118314114252e/PeerCastStation/PeerCastStation.PCP/PCPOutputStream.cs#L595

### Shutdown について

- Drop で使うので Shutdown の通知は send がブロッキングタスク（というか同期関数）で送信できないとだめ
  - Shutdown の通知も待ちたいところ・・・

# こうすれば効率化できるのでは？

- 決まり事のパケットはもっとまとめるべき
  - 例えば、CHAN パケットには ID が含まれるようだけど、いちいち ID パケットに包んでいる
    - そもそもコネクション毎に流れるデータが決まっているのに、それは必要か？
  - つまるところ ID でパケットの中身は決まるのである
