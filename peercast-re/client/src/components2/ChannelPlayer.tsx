import flvjs from "flv.js"
// import { RespChannel } from "@re-api"

import {
  Dialog,
  DialogTrigger,
  DialogContent,
  DialogTitle,
  DialogDescription,
  DialogHeader,
} from "./ui/dialog"
import { channel } from "diagnostics_channel"
import React from "react"

interface ChannelPlayerProps {
  className?: string
  channel: any
}

const ChannelPlayer: React.FC<ChannelPlayerProps> = ({ channel, className }) => {
  const videoRef = React.useRef<HTMLVideoElement>(null);

  React.useEffect(() => {
    const player = flvjs.createPlayer(
      {
        type: "flv",
        isLive: true,
        hasAudio: true,
        hasVideo: true,
        url: `http://localhost:17144/stream/${channel.id}`,
        // ...props.flvMediaSourceOptions,
      },
      {
        // stashInitialSize: stashInitialSize,
        // enableStashBuffer: enableStashBuffer,
        // ...props.flvConfig,
      },
    )

    player.attachMediaElement(videoRef.current!)
    player.load()
    player.play()
    // player.on("error", (err) => {
    //   props.errorCallback?.(err);
    // });
  }, [])

  return (
    <div className={className}>
      <video
      controls={true}
      muted={true}
      ref={videoRef}
      // style={{height, width}}
      />
    </div>
  )
}

const DialogChannelPlayer: React.FC<ChannelPlayerProps> = ({ channel, className }) => {
  return (
    <Dialog>
      <DialogTrigger className={className}>Open</DialogTrigger>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{channel.info.name}</DialogTitle>
          <DialogDescription></DialogDescription>
        </DialogHeader>
        <ChannelPlayer channel={channel} />
      </DialogContent>
    </Dialog>
  )
}

export default ChannelPlayer
export { DialogChannelPlayer }
