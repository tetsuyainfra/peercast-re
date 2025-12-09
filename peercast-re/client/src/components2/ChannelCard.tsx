import { CodeSandboxLogoIcon, PlayIcon } from "@radix-ui/react-icons"
// import { deleteChannelsByBroadcastId, RespChannel } from "@re-api"
import { Button } from "./ui/button"
import { Card, CardContent, CardHeader, CardTitle } from "./ui/card"
import { useState } from "react"
import { DialogChannelPlayer } from "./ChannelPlayer"
import { string } from "zod"
import { play_url } from "@/lib/api"

interface ChannelCardProps {
  channel: any
}

const ChannelCard: React.FC<ChannelCardProps> = ({ channel }) => {
  let [playDisable, setPlayDisable] = useState(false)
  const playButton = (evt: React.MouseEvent<HTMLButtonElement>) => {
    setPlayDisable(true)
    window.open(play_url(channel.id))
    setPlayDisable(false)
    evt.preventDefault()
  }

  const stopButton = (evt: React.MouseEvent<HTMLButtonElement>) => {
    console.log(evt);

    // (async function () {
    //   await deleteChannelsByBroadcastId({ path: { broadcast_id: "a" } }).then((v) => {
    //     window.location.reload(),
    //       (err: any) => {
    //         window.location.reload()
    //       }
    //   })
    // })

    evt.preventDefault()
  }

  return (
    <Card>
      <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
        <CardTitle className="text-sm font-medium">
          {channel.info.name}
          <span className="text-right">{channel.id}</span>
        </CardTitle>
      </CardHeader>
      <CardContent>
        <div className="flex justify-between space-x-4">
          {/* <!-- --> */}
          <div className="flex space-x-4">
            <div>
              <p className="text-sm font-medium leading-none">
                {channel.info.genre} - {channel.info.desc}
              </p>
              <p className="text-sm text-muted-foreground">"{channel.info.comment}"</p>
            </div>
          </div>

          {/* <!-- 右側 --> */}
          <div className="">
            <Button variant="secondary" className="mr-4" onClick={stopButton}>
              Drop
              {/* <Loader2 className="mr-2 h-4 w-4 animate-spin" /> */}
            </Button>
            {/* <Button variant="secondary" className="mr-4">
              <Loader2 className="mr-2 h-4 w-4 animate-spin" />
            </Button> */}
            <Button variant="secondary" className="mr-4" asChild>
              <DialogChannelPlayer channel={channel} />
            </Button>
            <Button asChild>
              <a href={play_url(channel.id)}>再生</a>
            </Button>
          </div>
        </div>
      </CardContent>
    </Card>
  )
}

export default ChannelCard
