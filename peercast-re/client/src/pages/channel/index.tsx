import { JsonChannel } from "@/_gen-api"
import ChannelLayout from "@/layouts/channelLayout"
import { useEffect, useRef } from "react"
import { useLoaderData, useOutletContext } from "react-router"

import flvjs from "flv.js"

type FlvPlayerProps = {
  url: string
  width?: number
  height?: number
}

const FlvPlayer: React.FC<FlvPlayerProps> = ({ url, width = 640, height = 360 }) => {
  const videoRef = useRef(null)
  const playerRef = useRef<flvjs.Player | null>(null)

  useEffect(() => {
    if (typeof window === 'undefined') return; // SSR 対策
    if (!flvjs.isSupported()) {
      console.warn("flv.js is not supported in this browser.")
      return
    }
    if (!videoRef.current) return

    const player = flvjs.createPlayer({
      type: "flv",
      url, // 例: 'http://example.com/live/stream.flv'
      isLive: true, // ライブなら true、VOD なら false
      // その他オプション: cors, withCredentials など
    })

    player.attachMediaElement(videoRef.current)
    player.load()
    // 自動再生はブラウザによって返される Promise を扱う
    player.play()!.catch((error) => {
      console.error("Error attempting to play the video:", error)
    })

    playerRef.current = player

    // エラーやイベントハンドリングの例
    player.on(flvjs.Events.ERROR, (type, detail) => {
      console.error("flvjs error", type, detail)
    })

    return () => {
      if (playerRef.current) {
        console.info("Destroying flv.js player")
        playerRef.current.unload()
        playerRef.current.detachMediaElement()
        playerRef.current.destroy()
        playerRef.current = null
      }
    }
  }, [url])

  return (
    <video
      ref={videoRef}
      width={width}
      height={height}
      controls
      muted // 自動再生したい場合は muted を付ける
    />
  )
}

export default function Channel() {
  const { channel } = useLoaderData<{ channel: JsonChannel }>()
  console.info("Channel page loader data:", channel)

  const pageTitle = `${channel.desc} - ${channel.name}`

  return (
    <ChannelLayout pageTitle={pageTitle}>
      <div>
        {channel.desc} - {channel.name}
        <FlvPlayer url={`/stream/${channel.id}`} />
      </div>
    </ChannelLayout>
  )
}
