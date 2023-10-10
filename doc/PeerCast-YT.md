
# PeerCast-YT コードリーディングメモ
LICENCE：GPLv2 (PeCaYTがそうだからコードが含まれるこのメモもそれに従います)


## 1. Incommingな通信待ち受け
-
```cpp
// ソケットでの待ち受けを行う SERVER サーバントのメインプロシージャ。
int Servent::serverProc(ThreadInfo *thread)
{ ...
            Servent *ns = servMgr->allocServent();
            ...
            ns->initIncoming(cs, sv->allow);

// -----------------------------------
// クライアントとの対話を開始する。
void Servent::initIncoming(std::shared_ptr<ClientSocket> s, unsigned int a)
{ ...
        thread.func = incomingProc;
    if (!sys->startThread(&thread))

// -------------------------------------------------------------
// SERVER サーバントから起動されたサーバントのメインプロシージャ
int Servent::incomingProc(ThreadInfo *thread)
{ ...
        sv->handshakeIncoming();
```

## 1.1 PCP通信
```cpp
// -----------------------------------
void Servent::handshakeIncoming()
{...
    handshakeHTTP(http, isHTTP);
```

```cpp
// -----------------------------------
void Servent::handshakeHTTP(HTTP &http, bool isHTTP)
{ ...
{
    if (http.isRequest("GET /"))
    {   # /streamはここ！
        handshakeGET(http);          
    }else if (http.isRequest("POST /"))
    {
        handshakePOST(http);
    }else if (http.isRequest(PCX_PCP_CONNECT)) // "pcp"
    {
        // CIN

        if (!isAllowed(ALLOW_NETWORK) || !isFiltered(ServFilter::F_NETWORK))
            throw HTTPException(HTTP_SC_UNAVAILABLE, 503);

        # pcp\n~はここ！
        processIncomingPCP(true);
        // リレーがフル、既に接続があった、offair(rootモード|放送中)ならリレー候補を返すっぽい？
        // 有効なChannelIDを返すとは限らないよね・・・どうなの？
        if (unavailable || alreadyConnected || offair)
```

/[stream|channel]/CHANNEL_IDのアクセス
```cpp
https://github.com/plonk/peercast-yt/blob/787be6405cc2d82a5d26c0023aaa5d1973c13802/core/common/servhs.cpp#L271
// -----------------------------------
void Servent::handshakeGET(HTTP &http)
{
    ...
    }else if (strncmp(fn, "/stream/", 8) == 0)
    {
        // ストリーム

        if (!sock->host.isLocalhost())
            if (!isAllowed(ALLOW_DIRECT) || !isFiltered(ServFilter::F_DIRECT))
                throw HTTPException(HTTP_SC_UNAVAILABLE, 503);

        triggerChannel(fn+8, ChanInfo::SP_HTTP, isPrivate() || hasValidAuthToken(fn+8));
    }else if (strncmp(fn, "/channel/", 9) == 0)
    {
        if (!sock->host.isLocalhost())
            if (!isAllowed(ALLOW_NETWORK) || !isFiltered(ServFilter::F_NETWORK))
                throw HTTPException(HTTP_SC_UNAVAILABLE, 503);

        triggerChannel(fn+9, ChanInfo::SP_PCP, false);
```

```cpp
https://github.com/plonk/peercast-yt/blob/787be6405cc2d82a5d26c0023aaa5d1973c13802/core/common/servhs.cpp#L704
// -----------------------------------
// リレー接続、あるいはダイレクト接続に str で指定されたチャンネルのス
// トリームを流す。relay が true ならば、チャンネルが無かったり受信中
// でなくても、チャンネルを受信中の状態にしようとする。
void Servent::triggerChannel(char *str, ChanInfo::PROTOCOL proto, bool relay)
{
    ChanInfo info;

    servMgr->getChannel(str, info, relay);

    if (proto == ChanInfo::SP_PCP)
        type = T_RELAY;
    else
        type = T_DIRECT;

    outputProtocol = proto;

    processStream(info);
}
```

Streamの処理開始
```cpp
https://github.com/plonk/peercast-yt/blob/787be6405cc2d82a5d26c0023aaa5d1973c13802/core/common/servent.cpp#L1605
// -----------------------------------
void Servent::processStream(ChanInfo &chanInfo)
{
    setStatus(S_HANDSHAKE);

    // ハンドシェイク開始！
    if (!handshakeStream(chanInfo))
        return;

    ASSERT(chanID.isSet());

    if (chanInfo.id.isSet())
    {
        chanID = chanInfo.id;

        LOG_INFO("Sending channel: %s ", ChanInfo::getProtocolStr(outputProtocol));

        if (!waitForChannelHeader(chanInfo))
            throw StreamException("Channel not ready");

        servMgr->totalStreams++;

        auto ch = chanMgr->findChannelByID(chanID);
        if (!ch)
            throw StreamException("Channel not found");

        if (outputProtocol == ChanInfo::SP_HTTP)
        {
            if ((addMetadata) && (chanMgr->icyMetaInterval))
                sendRawMetaChannel(chanMgr->icyMetaInterval);
            else
                sendRawChannel(true, true);
        }else if (outputProtocol == ChanInfo::SP_MMS)
        {
            if (nsSwitchNum)
            {
                sendRawChannel(true, true);
            }else
            {
                sendRawChannel(true, false);
            }
        }else if (outputProtocol  == ChanInfo::SP_PCP)
        {
            sendPCPChannel();
        }
    }

    setStatus(S_CLOSING);
}
```

```cpp
https://github.com/plonk/peercast-yt/blob/787be6405cc2d82a5d26c0023aaa5d1973c13802/core/common/servent.cpp#L870
// -----------------------------------
// "/stream/<channel ID>" あるいは "/channel/<channel ID>" エンドポイ
// ントへの要求に対する反応。
bool Servent::handshakeStream(ChanInfo &chanInfo)
{
    handshakeStream_readHeaders(gotPCP, reqPos, nsSwitchNum);
    handshakeStream_changeOutputProtocol(gotPCP, chanInfo);

    bool chanReady = false;

    auto ch = chanMgr->findChannelByID(chanInfo.id);


    // ch : channel infomation
    // chl :  channel host list
    return handshakeStream_returnResponse(gotPCP, chanReady, ch, chl.get(), chanInfo);

```

HTTPヘッダ読み込み
```cpp
// -----------------------------------
// HTTP ヘッダーを読み込み、gotPCP, reqPos, nsSwitchNum, this->addMetaData,
// this->agent を設定する。
void Servent::handshakeStream_readHeaders(bool& gotPCP, unsigned int& reqPos, int& nsSwitchNum)

```

HTTPヘッダ読み込んで切り替え
```cpp
// -----------------------------------
// 状況に応じて this->outputProtocol を設定する。
void Servent::handshakeStream_changeOutputProtocol(bool gotPCP, const ChanInfo& chanInfo)
{
    // WMV ならば MMS(MMSH)
    if (outputProtocol == ChanInfo::SP_HTTP)
    {
        if  ( (chanInfo.srcProtocol == ChanInfo::SP_MMS)
              || (chanInfo.contentType == ChanInfo::T_WMA)
              || (chanInfo.contentType == ChanInfo::T_WMV)
              || (chanInfo.contentType == ChanInfo::T_ASX)
            )
        outputProtocol = ChanInfo::SP_MMS;
    }
}
```

```cpp
// https://github.com/plonk/peercast-yt/blob/787be6405cc2d82a5d26c0023aaa5d1973c13802/core/common/servent.cpp#L808
// -----------------------------------
bool Servent::handshakeStream_returnResponse(bool gotPCP,
                                             bool chanReady, // ストリーム可能である。
                                             std::shared_ptr<Channel> ch,
                                             ChanHitList* chl,
                                             const ChanInfo& chanInfo)
{
    Host rhost = sock->host;
    AtomStream atom(*sock);

    if (!chl)
    {
        sock->writeLine(HTTP_SC_NOTFOUND);
        sock->writeLine("");
        LOG_DEBUG("Sending channel not found");
        return false;
    }else if (!chanReady)       // cannot stream
    {
        if (outputProtocol == ChanInfo::SP_PCP)    // relay stream
        {
            sock->writeLine(HTTP_SC_UNAVAILABLE);
            sock->writeLineF("%s %s", HTTP_HS_CONTENT, MIME_XPCP);
            sock->writeLine("");

            handshakeIncomingPCP(atom, rhost, remoteID, agent);

            LOG_DEBUG("Sending channel unavailable");

            handshakeStream_returnHits(atom, chanInfo.id, chl, rhost);
            return false;
        }else                                      // direct stream
        {
            LOG_DEBUG("Sending channel unavailable");
            sock->writeLine(HTTP_SC_UNAVAILABLE);
            sock->writeLine("");
            return false;
        }
    }else
    {
        handshakeStream_returnStreamHeaders(atom, ch, chanInfo);

        if (gotPCP)
        {
            handshakeIncomingPCP(atom, rhost, remoteID, agent);
            atom.writeInt(PCP_OK, 0);
        }
        return true;
    }
}

```

pcp\nのエントリーポイント
```cpp
// https://github.com/plonk/peercast-yt/blob/787be6405cc2d82a5d26c0023aaa5d1973c13802/core/common/servent.cpp#L1250
// ------------------------------------------------------------------
// コントロール・インの処理。通信の状態は、"pcp\n" を読み込んだ後。
// suggestOthers は常に true が渡される。
void Servent::processIncomingPCP(bool suggestOthers)
{
    PCPStream::readVersion(*sock);// ここでpcp\nに続く0x0001, 0x0001を読み込んでいる

    AtomStream atom(*sock);
    Host rhost = sock->host;

    handshakeIncomingPCP(atom, rhost, remoteID, agent);

    bool alreadyConnected = (servMgr->findConnection(Servent::T_COUT, remoteID)!=NULL) ||
                            (servMgr->findConnection(Servent::T_CIN,  remoteID)!=NULL);
    bool unavailable      = servMgr->controlInFull();
    bool offair           = !servMgr->isRoot && !chanMgr->isBroadcasting();

    char rstr[64];
    strcpy(rstr, rhost.str().c_str());

    // 接続を断わる場合の処理。コントロール接続数が上限に達しているか、
    // リモートホストとのコントロール接続が既にあるか、自分は放送中の
    // トラッカーではない。
    if (unavailable || alreadyConnected || offair)
    {
        int error;

        if (alreadyConnected)
            error = PCP_ERROR_QUIT+PCP_ERROR_ALREADYCONNECTED;
        else if (unavailable)
            error = PCP_ERROR_QUIT+PCP_ERROR_UNAVAILABLE;
        else if (offair)
            error = PCP_ERROR_QUIT+PCP_ERROR_OFFAIR;
        else
            error = PCP_ERROR_QUIT;

        if (suggestOthers)
        {
            ChanHit best;
            ChanHitSearch chs;

            int cnt=0;
            for (int i=0; i<8; i++)
            {
                best.init();

                // find best hit on this network
                if (!rhost.globalIP())
                {
                    chs.init();
                    chs.matchHost = servMgr->serverHost;
                    chs.waitDelay = 2;
                    chs.excludeID = remoteID;
                    chs.trackersOnly = true;
                    chs.useBusyControls = false;
                    if (chanMgr->pickHits(chs))
                        best = chs.best[0];
                }

                // find best hit on same network
                if (!best.host.ip)
                {
                    chs.init();
                    chs.matchHost = rhost;
                    chs.waitDelay = 2;
                    chs.excludeID = remoteID;
                    chs.trackersOnly = true;
                    chs.useBusyControls = false;
                    if (chanMgr->pickHits(chs))
                        best = chs.best[0];
                }

                // else find best hit on other networks
                if (!best.host.ip)
                {
                    chs.init();
                    chs.waitDelay = 2;
                    chs.excludeID = remoteID;
                    chs.trackersOnly = true;
                    chs.useBusyControls = false;
                    if (chanMgr->pickHits(chs))
                        best = chs.best[0];
                }

                if (!best.host.ip)
                    break;

                best.writeAtoms(atom, GnuID());
                cnt++;
            }

            if (cnt)
            {
                LOG_DEBUG("Sent %d tracker(s) to %s", cnt, rstr);
            }else if (rhost.port)
            {
                // send push request to best firewalled tracker on other network
                chs.init();
                chs.waitDelay = 30;
                chs.excludeID = remoteID;
                chs.trackersOnly = true;
                chs.useFirewalled = true;
                chs.useBusyControls = false;
                if (chanMgr->pickHits(chs))
                {
                    best = chs.best[0];
                    int cnt = servMgr->broadcastPushRequest(best, rhost, GnuID(), Servent::T_CIN);
                    LOG_DEBUG("Broadcasted tracker push request to %d clients for %s", cnt, rstr);
                }
            }else
            {
                LOG_DEBUG("No available trackers");
            }
        }

        LOG_ERROR("Sending QUIT to incoming: %d", error);

        atom.writeInt(PCP_QUIT, error);
        return;
    }

    // ここから接続扱いっぽい
    type = T_CIN;
    setStatus(S_CONNECTED);

    atom.writeInt(PCP_OK, 0);

    // ask for update
    atom.writeParent(PCP_ROOT, 1);
        atom.writeParent(PCP_ROOT_UPDATE, 0);

    pcpStream = new PCPStream(remoteID);

    int error = 0;
    BroadcastState bcs;
    while (!error && thread.active() && !sock->eof())
    {
        error = pcpStream->readPacket(*sock, bcs);
        sys->sleepIdle();

        if (!servMgr->isRoot && !chanMgr->isBroadcasting())
            error = PCP_ERROR_OFFAIR;
        if (peercastInst->isQuitting)
            error = PCP_ERROR_SHUTDOWN;
    }

    pcpStream->flush(*sock);

    error += PCP_ERROR_QUIT;
    atom.writeInt(PCP_QUIT, error);

    LOG_DEBUG("PCP Incoming to %s closed: %d", rstr, error);
}


```

```cpp readVersion
https://github.com/plonk/peercast-yt/blob/787be6405cc2d82a5d26c0023aaa5d1973c13802/core/common/pcp.cpp#L41
// ------------------------------------------
void PCPStream::readVersion(Stream &in)
{
    int len = in.readInt();

    if (len != 4)
        throw StreamException("Invalid PCP");

    int ver = in.readInt();

    LOG_DEBUG("PCP ver: %d", ver);
}
```

```cpp
// https://github.com/plonk/peercast-yt/blob/787be6405cc2d82a5d26c0023aaa5d1973c13802/core/common/servent.cpp#L1142C15-L1142C35
// -------------------------------------------------------------------
// PCPハンドシェイク。HELO, OLEH。グローバルIP、ファイアウォールチェッ
// ク。
void Servent::handshakeIncomingPCP(AtomStream &atom, Host &rhost, GnuID &rid, String &agent)
{
    int numc, numd;
    ID4 id = atom.read(numc, numd);

    // 必ず一つ目のパケットはPCP_HELOを期待している
    if (id != PCP_HELO)
    {
        LOG_DEBUG("PCP incoming reply: %s", id.getString().str());
        atom.writeInt(PCP_QUIT, PCP_ERROR_QUIT+PCP_ERROR_BADRESPONSE);
        throw StreamException("Got unexpected PCP response");
    }

    char arg[64];

    ID4 osType;

    int version=0;

    int pingPort=0;

    GnuID bcID;
    GnuID clientID;

    rhost.port = 0;

    for (int i=0; i<numc; i++)
    {
        int c, dlen;
        ID4 id = atom.read(c, dlen);

        if (id == PCP_HELO_AGENT)
        {
            atom.readString(arg, sizeof(arg), dlen);
            agent.set(arg);
        }else if (id == PCP_HELO_VERSION)
        {
            version = atom.readInt();
        }else if (id == PCP_HELO_SESSIONID)
        {
            atom.readBytes(rid.id, 16);
            if (rid.isSame(servMgr->sessionID))
                throw StreamException("Servent loopback");
        }else if (id == PCP_HELO_BCID)
        {
            atom.readBytes(bcID.id, 16);
        }else if (id == PCP_HELO_OSTYPE)
        {
            osType = atom.readInt();
        }else if (id == PCP_HELO_PORT)
        {
            rhost.port = atom.readShort();
        }else if (id == PCP_HELO_PING)
        {
            pingPort = atom.readShort();
        }else
        {
            LOG_DEBUG("PCP handshake skip: %s", id.getString().str());
            atom.skip(c, dlen);
        }
    }

    if (version)
        LOG_DEBUG("Incoming PCP is %s : v%d", agent.cstr(), version);

    if (!rhost.globalIP() && servMgr->serverHost.globalIP())
        rhost.ip = servMgr->serverHost.ip;

    if (pingPort)
    {
        LOG_DEBUG("Incoming firewalled test request: %s ", rhost.str().c_str());
        rhost.port = pingPort;
        if (!rhost.globalIP() || !pingHost(rhost, rid))
            rhost.port = 0;
    }

    // HTTPでもpcp\nでもOLEHの内容は同じ
    atom.writeParent(PCP_OLEH, 5);
        atom.writeString(PCP_HELO_AGENT, PCX_AGENT);
        atom.writeBytes(PCP_HELO_SESSIONID, servMgr->sessionID.id, 16);
        atom.writeInt(PCP_HELO_VERSION, PCP_CLIENT_VERSION);
        atom.writeAddress(PCP_HELO_REMOTEIP, rhost.ip);
        atom.writeShort(PCP_HELO_PORT, rhost.port);

    if (version)
    {
        // バージョンが低ければ切る
        if (version < PCP_CLIENT_MINVERSION)
        {
            atom.writeInt(PCP_QUIT, PCP_ERROR_QUIT+PCP_ERROR_BADAGENT);
            throw StreamException("Agent is not valid");
        }
    }

     // REMOTE SESSION＿IDが無ければ切る
    if (!rid.isSet())
    {
        atom.writeInt(PCP_QUIT, PCP_ERROR_QUIT+PCP_ERROR_NOTIDENTIFIED);
        throw StreamException("Remote host not identified");
    }

    // RootモードならRootのアトムを書く
    if (servMgr->isRoot)
    {
        servMgr->writeRootAtoms(atom, false);
    }

    LOG_DEBUG("PCP Incoming handshake complete.");
}
```

RootのAtom
```cpp
https://github.com/plonk/peercast-yt/blob/787be6405cc2d82a5d26c0023aaa5d1973c13802/core/common/servmgr.cpp#L1895C41-L1895C41
// --------------------------------------------------
void ServMgr::writeRootAtoms(AtomStream &atom, bool getUpdate) 
// Githubで検索した限り、getUpdate==falseでしか呼ばれてない
{
    atom.writeParent(PCP_ROOT, 5 + (getUpdate?1:0));
        atom.writeInt(PCP_ROOT_UPDINT, chanMgr->hostUpdateInterval);
        atom.writeString(PCP_ROOT_URL, "download.php");
        atom.writeInt(PCP_ROOT_CHECKVER, PCP_ROOT_VERSION);
        atom.writeInt(PCP_ROOT_NEXT, chanMgr->hostUpdateInterval);
        atom.writeString(PCP_MESG_ASCII, rootMsg.cstr());
        if (getUpdate)  
            atom.writeParent(PCP_ROOT_UPDATE, 0);
}

```

Pingチェック
https://github.com/plonk/peercast-yt/blob/787be6405cc2d82a5d26c0023aaa5d1973c13802/core/common/servent.cpp#L471


## 1.1 TrackerからRooteへのPCP\n接続

エンドポイントっていうかスタート地点はココ
```cpp
// https://github.com/plonk/peercast-yt/blob/787be6405cc2d82a5d26c0023aaa5d1973c13802/core/common/servent.cpp#L1401
int Servent::outgoingProc(ThreadInfo *thread)
{
    sys->setThreadName("COUT");

    LOG_DEBUG("COUT started");

    // serventのインスタンスをポイントに写しとる
    Servent *sv = (Servent*)thread->data;
    Defer cb([sv]() { sv->kill(); });

    sv->pcpStream = new PCPStream(GnuID());

    // threadが有効な限り実行
    while (sv->thread.active())
    {
        sv->setStatus(S_WAIT);

        //放送中なら実行
        if (chanMgr->isBroadcasting() && servMgr->autoServe)
        {
            ChanHit bestHit;
            ChanHitSearch chs;

            // なにがしたいんだろ？
            do
            {
                bestHit.init();

                if (servMgr->rootHost.isEmpty())
                    break;

                // GIVってので使ってる・・・PUSH 配信と関係ある？
                // よくわきゃらんぴん！
                if (sv->pushSock)
                {
                    sv->sock = sv->pushSock;
                    sv->pushSock = NULL;
                    bestHit.host = sv->sock->host;
                    break;
                }

                // GnuID = 0x00_u128で検索してる
                auto chl = chanMgr->findHitListByID(GnuID());
                // ID 0x00のが存在するってこと？
                // WEB-UIを見るとネットワークの指定が1つしかない GnuID == 0をRootにしているのかな？
                if (chl)
                { 
                    // find local tracker
                    chs.init(); // ChannelHitSearchを初期化して・・・
                    chs.matchHost = servMgr->serverHost;
                    chs.waitDelay = MIN_TRACKER_RETRY;
                    chs.excludeID = servMgr->sessionID;
                    chs.trackersOnly = true;

                    // 絞り込みしてるだけ？
                    if (!chl->pickHits(chs))
                    {
                        // else find global tracker
                        chs.init();
                        chs.waitDelay = MIN_TRACKER_RETRY;
                        chs.excludeID = servMgr->sessionID;
                        chs.trackersOnly = true;
                        chl->pickHits(chs);
                    }

                    if (chs.numResults)
                    {
                        bestHit = chs.best[0];
                    }
                }

                unsigned int ctime = sys->getTime();

                // あーもしかしてYPのIP探してるだけか？
                if ((!bestHit.host.ip) && ((ctime-chanMgr->lastYPConnect) > MIN_YP_RETRY))
                {
                    bestHit.host.fromStrName(servMgr->rootHost.cstr(), DEFAULT_PORT);
                    bestHit.yp = true;
                    chanMgr->lastYPConnect = ctime;
                }
                sys->sleepIdle();
            }while (!bestHit.host.ip && (sv->thread.active()));
            // hostのIPが確定してthreadが有効ならこのloopから逃れる

            if (!bestHit.host.ip)       // give up
            {
                LOG_ERROR("COUT giving up");
                break;
            }

            const std::string ipStr = bestHit.host.str();

            int error=0;
            try
            {
                LOG_DEBUG("COUT to %s: Connecting..", ipStr.c_str());

                if (!sv->sock)
                {
                    sv->setStatus(S_CONNECTING);
                    sv->sock = sys->createSocket();
                    if (!sv->sock)
                        throw StreamException("Unable to create socket");
                    sv->sock->open(bestHit.host); // IP使って接続
                    sv->sock->connect();
                }

                sv->sock->setReadTimeout(30000);
                AtomStream atom(*sv->sock);

                sv->setStatus(S_HANDSHAKE);

                // ここから接続！
                Host rhost = sv->sock->host;
                atom.writeInt(PCP_CONNECT, 1);
                handshakeOutgoingPCP(atom, rhost, sv->remoteID, sv->agent, bestHit.yp); // HELO投げてOLEH受信してるだけ

                sv->setStatus(S_CONNECTED);

                LOG_DEBUG("COUT to %s: OK", ipStr.c_str());

                // REMOTE SESSION_IDを記録してる
                sv->pcpStream->init(sv->remoteID);

                BroadcastState bcs;
                error = 0;

                // ここで接続しっぱなしにしている
                // autoServeはbool値っぽい
                while (!error && sv->thread.active() && !sv->sock->eof() && servMgr->autoServe)
                {
                    // https://github.com/plonk/peercast-yt/blob/787be6405cc2d82a5d26c0023aaa5d1973c13802/core/common/pcp.cpp#L95
                    // ここを読むと
                    error = sv->pcpStream->readPacket(*sv->sock, bcs);

                    sys->sleepIdle(); // 標準で10秒スリープ

                    if (!chanMgr->isBroadcasting())
                        error = PCP_ERROR_OFFAIR;
                    if (peercastInst->isQuitting)
                        error = PCP_ERROR_SHUTDOWN;

                    if (sv->pcpStream->nextRootPacket)
                        if (sys->getTime() > (sv->pcpStream->nextRootPacket+30))
                            error = PCP_ERROR_NOROOT;
                }
                sv->setStatus(S_CLOSING);

                sv->pcpStream->flush(*sv->sock);

                error += PCP_ERROR_QUIT;
                atom.writeInt(PCP_QUIT, error);

                LOG_ERROR("COUT to %s closed: %d", ipStr.c_str(), error);
            }catch (TimeoutException &e)
            {
                LOG_ERROR("COUT to %s: timeout (%s)", ipStr.c_str(), e.msg);
                sv->setStatus(S_TIMEOUT);
            }catch (StreamException &e)
            {
                LOG_ERROR("COUT to %s: %s", ipStr.c_str(), e.msg);
                sv->setStatus(S_ERROR);
            }

            try
            {
                if (sv->sock)
                {
                    sv->sock->close();
                    sv->sock = NULL;
                }
            }catch (StreamException &) {}

            // don`t discard this hit if we caused the disconnect (stopped broadcasting)
            if (error != (PCP_ERROR_QUIT+PCP_ERROR_OFFAIR))
                chanMgr->deadHit(bestHit);
        }

        sys->sleepIdle(); // defaultで10秒idle
    }

    LOG_DEBUG("COUT ended");

    return 0;
}
```

呼び出しもとはここ
```cpp
// https://github.com/plonk/peercast-yt/blob/787be6405cc2d82a5d26c0023aaa5d1973c13802/core/common/servmgr.cpp#L209
// -----------------------------------
void    ServMgr::connectBroadcaster()
{
    if (!rootHost.isEmpty())
    {
        if (!numUsed(Servent::T_COUT))
        {
            Servent *sv = allocServent();
            if (sv)
            {
                sv->initOutgoing(Servent::T_COUT);   ****** 1 *****
                sys->sleep(3000);
            }
        }
    }
}

// -----------------------------------
void Servent::initOutgoing(TYPE ty)
{
    std::lock_guard<std::recursive_mutex> cs(lock);
    try
    {
        checkFree();

        type = ty;

        thread.data = this;
        thread.func = outgoingProc;

        if (!sys->startThread(&thread))
            throw StreamException("Can`t start thread");
    }catch (StreamException &e)
    {
        LOG_ERROR("Unable to start outgoing: %s", e.msg);
        kill();
    }
}


こっちは内部をコメントアウトされてる
// --------------------------------------------------
int ServMgr::clientProc(ThreadInfo *thread)
```


```cpp
// https://github.com/plonk/peercast-yt/blob/787be6405cc2d82a5d26c0023aaa5d1973c13802/core/common/servent.cpp#L1027
// -----------------------------------
void Servent::handshakeOutgoingPCP(AtomStream &atom, Host &rhost, GnuID &rid, String &agent, bool isTrusted)
{
    int ipv = rhost.ip.isIPv4Mapped() ? 4 : 6;
    if (servMgr->flags.get("sendPortAtomWhenFirewallUnknown"))
    {
        bool sendPort = (servMgr->getFirewall(ipv) != ServMgr::FW_ON);
        bool testFW   = (servMgr->getFirewall(ipv) == ServMgr::FW_UNKNOWN);
        bool sendBCID = isTrusted && chanMgr->isBroadcasting();

        writeHeloAtom(atom, sendPort, testFW, sendBCID, servMgr->sessionID, servMgr->serverHost.port, chanMgr->broadcastID);
    }else
    {
        bool sendPort = (servMgr->getFirewall(ipv) == ServMgr::FW_OFF);
        bool testFW   = (servMgr->getFirewall(ipv) == ServMgr::FW_UNKNOWN);
        bool sendBCID = isTrusted && chanMgr->isBroadcasting();

        writeHeloAtom(atom, sendPort, testFW, sendBCID, servMgr->sessionID, servMgr->serverHost.port, chanMgr->broadcastID);
    }

    LOG_DEBUG("PCP outgoing waiting for OLEH..");

    int numc, numd;
    ID4 id = atom.read(numc, numd);
    if (id != PCP_OLEH)
    {
        LOG_DEBUG("PCP outgoing reply: %s", id.getString().str());
        atom.writeInt(PCP_QUIT, PCP_ERROR_QUIT + PCP_ERROR_BADRESPONSE);
        throw StreamException("Got unexpected PCP response");
    }

    char arg[64];

    GnuID clientID;
    rid.clear();
    int version = 0;
    int disable = 0;

    Host thisHost;

    // read OLEH response
    for (int i = 0; i < numc; i++)
    {
        int c, dlen;
        ID4 id = atom.read(c, dlen);

        if (id == PCP_HELO_AGENT)
        {
            atom.readString(arg, sizeof(arg), dlen);
            agent.set(arg);
        }else if (id == PCP_HELO_REMOTEIP)
        {
            thisHost.ip = atom.readAddress();
        }else if (id == PCP_HELO_PORT)
        {
            thisHost.port = atom.readShort();
        }else if (id == PCP_HELO_VERSION)
        {
            version = atom.readInt();
        }else if (id == PCP_HELO_DISABLE)
        {
            disable = atom.readInt();
        }else if (id == PCP_HELO_SESSIONID)
        {
            atom.readBytes(rid.id, 16);
            if (rid.isSame(servMgr->sessionID))
                throw StreamException("Servent loopback");
        }else
        {
            LOG_DEBUG("PCP handshake skip: %s", id.getString().str());
            atom.skip(c, dlen);
        }
    }

    // update server ip/firewall status
    if (isTrusted)
    {
        if (thisHost.isValid())
        {
            if ((servMgr->serverHost.ip != thisHost.ip) && (servMgr->forceIP.isEmpty()))
            {
                LOG_DEBUG("Got new ip: %s", thisHost.str().c_str());
                servMgr->updateIPAddress(thisHost.ip);
            }

            if (servMgr->getFirewall(ipv) == ServMgr::FW_UNKNOWN)
            {
                if (thisHost.port && thisHost.globalIP())
                    servMgr->setFirewall(ipv, ServMgr::FW_OFF);
                else
                    servMgr->setFirewall(ipv, ServMgr::FW_ON);
            }
        }

        if (disable == 1)
        {
            LOG_ERROR("client disabled: %d", disable);
            servMgr->isDisabled = true;
        }else
        {
            servMgr->isDisabled = false;
        }
    }

    if (!rid.isSet())
    {
        atom.writeInt(PCP_QUIT, PCP_ERROR_QUIT + PCP_ERROR_NOTIDENTIFIED);
        throw StreamException("Remote host not identified");
    }

    LOG_DEBUG("PCP Outgoing handshake complete.");
}
```

```cpp
int PCPStream::readPacket(Stream &in, BroadcastState &bcs)
{
    int error = PCP_ERROR_GENERAL;
    try
    {
        AtomStream atom(in);

        ChanPacket pack;
        MemoryStream mem(pack.data, sizeof(pack.data));
        AtomStream patom(mem);

        // send outward packets
        error = PCP_ERROR_WRITE;
        if (outData.numPending())
        {
            outData.readPacket(pack);
            pack.writeRaw(in);
        }
        error = PCP_ERROR_GENERAL;

        if (outData.willSkip())
        {
            error = PCP_ERROR_WRITE+PCP_ERROR_SKIP;
            throw StreamException("Send too slow");
        }

        error = PCP_ERROR_READ;
        // poll for new downward packet
        if (in.readReady())
        {
            int numc, numd;
            ID4 id;

            id = atom.read(numc, numd);

            mem.rewind();
            pack.len = patom.writeAtoms(id, in, numc, numd);
            pack.type = ChanPacket::T_PCP;

            inData.writePacket(pack);
        }
        error = PCP_ERROR_GENERAL;

        // process downward packets
        if (inData.numPending())
        {
            inData.readPacket(pack);

            mem.rewind();

            int numc, numd;
            ID4 id = patom.read(numc, numd);

            error = PCPStream::procAtom(patom, id, numc, numd, bcs); // ここで処理されてるっぽい！

            if (error)
                throw StreamException("PCP exception");
        }

        error = 0;
    }catch (StreamException &e)
    {
        LOG_ERROR("PCP readPacket: %s (%d)", e.msg, error);
    }

    return error;
}
```

ここでAtomを捌いている
```cpp
// https://github.com/plonk/peercast-yt/blob/787be6405cc2d82a5d26c0023aaa5d1973c13802/core/common/pcp.cpp#L652C1-L712C2
// ------------------------------------------
int PCPStream::procAtom(AtomStream &atom, ID4 id, int numc, int dlen, BroadcastState &bcs)
{
    int r = 0;

    if (id == PCP_CHAN)
    {
        readChanAtoms(atom, numc, bcs);
    }else if (id == PCP_ROOT)       // ここでRootを処理している
    {
        if (servMgr->isRoot)        // RootModeのPeerCastが受信することはない
            throw StreamException("Unauthorized root message");
        else
            readRootAtoms(atom, numc, bcs); // entry
    }else if (id == PCP_HOST)
    {
        readHostAtoms(atom, numc, bcs);
    }else if ((id == PCP_MESG_ASCII) || (id == PCP_MESG))       // PCP_MESG_ASCII to be depreciated
    {
        String msg;
        atom.readString(msg.data, sizeof(msg.data), dlen);
        LOG_DEBUG("PCP got text: %s", msg.cstr());
    }else if (id == PCP_BCST)
    {
        r = readBroadcastAtoms(atom, numc, bcs);
    }else if (id == PCP_HELO)
    {
        atom.skip(numc, dlen);
        atom.writeParent(PCP_OLEH, 1);
            atom.writeBytes(PCP_HELO_SESSIONID, servMgr->sessionID.id, 16);
    }else if (id == PCP_PUSH)
    {
        readPushAtoms(atom, numc, bcs);
    }else if (id == PCP_OK)
    {
        atom.readInt();
    }else if (id == PCP_QUIT)
    {
        r = atom.readInt();
        if (!r)
            r = PCP_ERROR_QUIT;
    }else if (id == PCP_ATOM)
    {
        for (int i=0; i<numc; i++)
        {
            int nc, nd;
            ID4 aid = atom.read(nc, nd);
            int ar = procAtom(atom, aid, nc, nd, bcs);
            if (ar)
                r = ar;
        }
    }else
    {
        LOG_ERROR("PCP unknown or misplaced atom: %s", id.getString().str());
        throw StreamException("Protocol error");
        //LOG_INFO("PCP skip: %s", id.getString().str());
        //atom.skip(numc, dlen);
    }

    return r;
}
```

```cpp
// ------------------------------------------
void PCPStream::readRootAtoms(AtomStream &atom, int numc, BroadcastState &bcs)
{
    String url;

    for (int i=0; i<numc; i++)
    {
        int c, d;
        ID4 id = atom.read(c, d);

        if (id == PCP_ROOT_UPDINT)
        {
            int si = atom.readInt();

            chanMgr->setUpdateInterval(si);
            LOG_DEBUG("PCP got new host update interval: %ds", si);
        }else if (id == PCP_ROOT_URL)
        {
            url = "http://www.peercast.org/";
            String loc;
            atom.readString(loc.data, sizeof(loc.data), d);
            url.append(loc);
        }else if (id == PCP_ROOT_CHECKVER)
        {
            unsigned int newVer = atom.readInt();
            if (newVer > PCP_CLIENT_VERSION)
            {
                Sys::strcpy_truncate(servMgr->downloadURL, sizeof(servMgr->downloadURL), url.cstr());
                peercast::notifyMessage(ServMgr::NT_UPGRADE, "There is a new version of PeerCast available, please click here to upgrade your client.");
            }
            LOG_DEBUG("PCP got version check: %d / %d", newVer, PCP_CLIENT_VERSION);
        }else if (id == PCP_ROOT_NEXT)
        {
            unsigned int time = atom.readInt();

            if (time)
            {
                unsigned int ctime = sys->getTime();
                nextRootPacket = ctime+time;
                LOG_DEBUG("PCP expecting next root packet in %us", time);
            }else
            {
                nextRootPacket = 0;
            }
        }else if (id == PCP_ROOT_UPDATE) // PCP_ROOT_UPDATE を 受信したらBroadcastすることになってるのか
        {
            atom.skip(c, d);

            chanMgr->broadcastTrackerUpdate(remoteID, true);
        }else if ((id == PCP_MESG_ASCII) || (id == PCP_MESG))           // PCP_MESG_ASCII to be depreciated
        {
            String newMsg;

            atom.readString(newMsg.data, sizeof(newMsg.data), d);
            if (!newMsg.isSame(servMgr->rootMsg.cstr()))
            {
                servMgr->rootMsg = newMsg;
                LOG_DEBUG("PCP got new root mesg: %s", servMgr->rootMsg.cstr());
                if (servMgr->rootMsg != "")
                    peercast::notifyMessage(ServMgr::NT_PEERCAST,
                                            (std::string(servMgr->rootHost.str()) + "「" + servMgr->rootMsg.cstr() + "」").c_str());
            }
        }else
        {
            LOG_DEBUG("PCP skip: %s, %d, %d", id.getString().str(), c, d);
            atom.skip(c, d);
        }
    }
}
```

```cpp
// -----------------------------------
void ChanMgr::broadcastTrackerUpdate(const GnuID &svID, bool force)
{
    auto c = channel;
    while (c)
    {
        if ( c->isActive() && c->isBroadcasting() )
            c->broadcastTrackerUpdate(svID, force);
        c = c->next;
    }
}
```

```cpp
// -----------------------------------
// トラッカーである自分からYPへの通知。
void Channel::broadcastTrackerUpdate(const GnuID &svID, bool force /* = false */)
{
    unsigned int ctime = sys->getTime();

    if (((ctime-lastTrackerUpdate) > 30) || (force))
    {
        ChanPacket pack;

        MemoryStream mem(pack.data, sizeof(pack.data));
        AtomStream atom(mem);

        writeTrackerUpdateAtom(atom);  // ここでATOMを作って

        pack.len = mem.pos;
        pack.type = ChanPacket::T_PCP;

        int cnt = servMgr->broadcastPacket(pack, GnuID(), servMgr->sessionID, svID, Servent::T_COUT); // 送信

        if (cnt)
        {
            LOG_DEBUG("Sent tracker update for %s to %d client(s)", info.name.cstr(), cnt);
            lastTrackerUpdate = ctime;
        }
    }
}
```

#### Broadcast ATOMの作り方
```
// -----------------------------------
void Channel::writeTrackerUpdateAtom(AtomStream& atom)
{
    auto chl = chanMgr->findHitListByID(info.id);
    if (!chl)
        throw StreamException("Broadcast channel has no hitlist");

    int numListeners = totalListeners();
    int numRelays = totalRelays();

    unsigned int oldp = rawData.getOldestPos();
    unsigned int newp = rawData.getLatestPos();

    ChanHit hit;
    hit.initLocal(numListeners, numRelays, info.numSkips, info.getUptime(), isPlaying(),
                  oldp, newp, canAddRelay(), this->sourceHost.host, (ipVersion == IP_V6));
    hit.tracker = true;

    atom.writeParent(PCP_BCST, 10);
        atom.writeChar(PCP_BCST_GROUP, PCP_BCST_GROUP_ROOT);
        atom.writeChar(PCP_BCST_HOPS, 0);
        atom.writeChar(PCP_BCST_TTL, 7);
        atom.writeBytes(PCP_BCST_FROM, servMgr->sessionID.id, 16);
        atom.writeInt(PCP_BCST_VERSION, PCP_CLIENT_VERSION);
        atom.writeInt(PCP_BCST_VERSION_VP, PCP_CLIENT_VERSION_VP);
        atom.writeBytes(PCP_BCST_VERSION_EX_PREFIX, PCP_CLIENT_VERSION_EX_PREFIX, 2);
        atom.writeShort(PCP_BCST_VERSION_EX_NUMBER, PCP_CLIENT_VERSION_EX_NUMBER);
        atom.writeParent(PCP_CHAN, 4);
            atom.writeBytes(PCP_CHAN_ID, info.id.id, 16);
            atom.writeBytes(PCP_CHAN_BCID, chanMgr->broadcastID.id, 16);
            info.writeInfoAtoms(atom);
            info.writeTrackAtoms(atom);
        hit.writeAtoms(atom, info.id);
}
```

```cpp
int ServMgr::broadcastPacket(ChanPacket &pack, const GnuID &chanID, const GnuID &srcID, const GnuID &destID, Servent::TYPE type)
{
    std::lock_guard<std::recursive_mutex> cs(lock);

    int cnt=0;

    Servent *sv = servents;
    while (sv)
    {
        if (sv->sendPacket(pack, chanID, srcID, destID, type))
            cnt++;
        sv=sv->next;
    }
    return cnt;
}
```

PCP_BCSTのATOMを読みだしている
```cpp
// ------------------------------------------
void PCPStream::readChanAtoms(AtomStream &atom, int numc, BroadcastState &bcs)
{
    std::shared_ptr<Channel> ch = NULL;
    ChanInfo newInfo;

    ch = chanMgr->findChannelByID(bcs.chanID);
    auto chl = chanMgr->findHitListByID(bcs.chanID);

    if (ch)
        newInfo = ch->info;
    else if (chl)
        newInfo = chl->info;

    for (int i = 0; i < numc; i++)
    {
        int c, d;
        ID4 id = atom.read(c, d);

        if ((id == PCP_CHAN_PKT) && (ch))
        {
            readPktAtoms(ch, atom, c, bcs);
        }else if (id == PCP_CHAN_INFO)
        {
            newInfo.readInfoAtoms(atom, c);
        }else if (id == PCP_CHAN_TRACK)
        {
            newInfo.readTrackAtoms(atom, c);
        }else if (id == PCP_CHAN_BCID)
        {
            atom.readBytes(newInfo.bcID.id, 16);
        }else if (id == PCP_CHAN_KEY)           // depreciated
        {
            atom.readBytes(newInfo.bcID.id, 16);
            newInfo.bcID.id[0] = 0;             // clear flags
        }else if (id == PCP_CHAN_ID)
        {
            atom.readBytes(newInfo.id.id, 16);

            ch = chanMgr->findChannelByID(newInfo.id);
            chl = chanMgr->findHitListByID(newInfo.id);
        }else
        {
            // IM50,51 対策。
            LOG_ERROR("PCP unknown or misplaced atom: %s, %d, %d", id.getString().str(), c, d);
            throw StreamException("Protocol error");

            //LOG_DEBUG("PCP skip: %s, %d, %d", id.getString().str(), c, d);
            //atom.skip(c, d);
        }
    }

    if (!chl)
        chl = chanMgr->addHitList(newInfo);

    if (chl)
    {
        chl->info.update(newInfo);

        if (!servMgr->chanLog.isEmpty())
        {
            try
            {
                FileStream file;
                file.openWriteAppend(servMgr->chanLog.cstr());
                XML::Node *rn = new XML::Node("update time=\"%u\"", sys->getTime());
                XML::Node *n = chl->info.createChannelXML();
                n->add(chl->createXML(false));
                n->add(chl->info.createTrackXML());
                rn->add(n);
                rn->write(file, 0);
                delete rn;
                file.close();
            }catch (StreamException &e)
            {
                LOG_ERROR("Unable to update channel log: %s", e.msg);
            }
        }
    }

    if (ch && !ch->isBroadcasting())
        ch->updateInfo(newInfo);
}
```



## 1.2 HTTP+PCP通信
