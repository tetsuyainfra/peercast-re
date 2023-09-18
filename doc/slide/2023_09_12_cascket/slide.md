---
marp: true
theme: "base"
paginate: true
---
# PeerCast RE:

tetsuyainfra
2023/09/12

---
# 目次
<!-- _class: toc -->

- What is RE:
- Why?
- Demo
- Where am go ?
- 質疑応答

<!-- ---
# What is RE:
![bg contain right](https://upload.wikimedia.org/wikipedia/commons/0/0d/Re-Horakhty.svg)

---
# Oops, It is Ra(Re)
![bg contain right:33%](https://upload.wikimedia.org/wikipedia/commons/0/0d/Re-Horakhty.svg)

Ra,𒊑𒀀, 𒊑𒅀 or Re was the ancient Egyptian deity of the sun.
ラーは古代エジプトの太陽の神$^{[1]}$

 -->

---
What is RE:1

# **Re:** $^{[2]}$

> 「バック、元の場所に戻る、再び、もう一度」という意味を持ち、～
> 「...新たに、反対に」という意味を持つ。

---
What is RE:2

# **R**ust **E**dition

---
# Why ?
先行アプリケーションあるよね？
- あります！(PeerCastStation, PeerCast-YT|IM@S|VP)

作りたかったので作った
- NAS+iOSで視聴できる環境作りたい（今回は間に合わず）


---
# Demo
1. 視聴
2. 配信


---
# Where am go ?
## 競合
Youtube, ツイキャス, ふわっち, ニコニコ

## 新規参入の壁
環境整備(インストール、掲示板、コメント表示)
安定しない視聴環境

## 高齢キャスト

---
# But...

---
# We have free.
俺達は自由だ！
  - 使う掲示板は自分で選ぶことができるし
  - なんでも配信できる
    - もちろん**法**が許す限り
  - 新規配信者もいなくはない。復帰勢もいる！

---
# 引用
- [1] https://en.wikipedia.org/wiki/Ra (画像含む)
- [2] https://www.etymonline.com/jp/word/re-

# ところで...
- PCP_CHAN_TRACK_GENRE
  - YT(なし), PeCaSt(あり)
- FLVストリームでAVCヘッダーの更新を考慮しないといけないの辛くない？

---
# 補足(PeerCast関係)

---
# じぶんについて
### 仕事なにしてるの？
- ひみつ(名は体を表す)

### どんな配信みてるの？
- 真田、〇ぐらさん、〇む打つ、〇lack Ch☆ミ

### どんなゲーム好き？
- 最近はローグレガシー2, Factorio

---
# 補足(技術関係)

---
# 今後やりたい（実装）ことは？
1. テストまともに作る(単体・結合・integration)
2. ROOTモード(YellowPageで使えるようにしたい)
3. MPEG-DASHで視聴できるにしたい
   1. もしくはWebRTC
4. TUIの作成 → 必要ないよなーたぶん・・・
5. なんかある？

---
# Future(Rust async/await)
- むずかしい・・・
  - そのままだと再帰呼び出しができない
    - async_recursion crateを使うといける
  - 並行処理が書きにくい
    - tokio::select!
      - 宣言的に書くので
    - futures_util::future::select_all
    - Threadでも同じことか
- 簡単
  - await忘れはcompilerが指摘してくれる
  - impl Futureはもっと難しいのでasync/await糖衣ありがたい
- とは言ってもThread版実装してないし性能比較はできない
