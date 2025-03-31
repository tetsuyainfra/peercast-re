## 注意

- Local, Server, Peer は全て Peercast Application である。
- Server は配信者の Peercast
- Local は視聴者の Peercast
- Peer は視聴者の Peercast である。ただし既に Relay している状態の者
  - Relay とは Server が配信している Channel データを中継している状態を指す

## 正常系

```mermaid
sequenceDiagram
participant local as Client
participant svr as Server

local ->> svr: GET /channel/[CHANNEL_ID]
svr -->> local: Status 200
local ->> svr: HELO ATOM
svr -->> local: OLEH ATOM
svr -->> local: OK ATOM
loop 配信終了まで
  svr -->> local: [CHAN | BROADCAST] ATOM
  local ->> svr: BROADCAST ATOM
end
svr -->> local: QUIT ATOM

```

## 異常系

```mermaid
sequenceDiagram
participant local as Client
participant svr as Server
participant peer1 as Peer1
participant peer2 as Peer2

local ->> svr: GET /channel/[CHANNEL_ID]
svr -->> local: Status "503"
local ->> svr: HELO ATOM
svr -->> local: OLEH ATOM
loop 接続終了まで(最大8個らしい)
  svr -->> local: HOST ATOM
end
svr -->> local: QUIT ATOM

local ->> local : select next peer

local ->> peer1: GET /channel/[CHANNEL_ID]
peer1 -->> local: Status "503"
Note over local,peer1: Peer情報をもらう


local ->> peer2: GET /channel/[CHANNEL_ID]
peer2 -->> local: Status 200
Note over local,peer2: 正常系の手続きへ

```

Caster ->> OutConnection: spawn new task
Local ->> Channel: connect with ChannelID
Channel ->> EntryPoint: true
OutConnection ->> Remote: http request to Peercast remote
OutConnection ->> Remote: handshake PCP to Peercast remote
Remote -->> OutConnection: OLEH and OK atom packet
loop Receiving Task
Remote -->> OutConnection: OLEAH and OK atom packet

end
