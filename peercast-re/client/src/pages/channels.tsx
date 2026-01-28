import React, { useEffect, useState } from "react"
import { env } from "process"
import MainLayout from "@/layouts/mainLayout"
// import { RespChannel } from "@re-api"
import { Link } from "react-router"
import { channelApi } from "@/api"

export default function Channels() {
  const urlInputId = React.useId()
  let [channels, channelsSet] = useState<any[]>([])
  useEffect(() => {
    ;(async () => {
      const { data, error } = await channelApi.list();
      console.info("listChannels", { data, error })
      channelsSet((data as any) || [])
    })()
    // (async () => {
    //   let api = new ChannelApi(api_config())
    //   await api.channelsGet().then(
    //     (channels) => {
    //       channelsSet(channels)
    //     },
    //     (err) => {
    //       console.info("channelsGet failed", err)
    //     },
    //   )
    // })()
  }, [])

  const addChannel = (evt: React.FormEvent<HTMLFormElement>) => {
    evt.preventDefault()
    let urlInput = (document.getElementById(urlInputId) as HTMLInputElement).value
    let url = new URL(urlInput)
    let id = url.pathname.split("/").at(-1) || ""
    let host = url.searchParams.get("tip") || ""
  }

  return (
    <MainLayout pageTitle="Channels">
      <div className="flex flex-col border-b border-gray-300 py-2">
        <h2 className="">▶️ EmptyDummy - No name</h2>
        <p className="flex flex-row gap-4 text-sm text-gray-600">
          <Link to={`/channel/00000000000000000000`} className="underline">
            Go to Channel Page
          </Link>
          <span>channel.comment</span>
          <span>channel.genre</span>
          <span>channel.id</span>
        </p>
      </div>
      {channels.map((channel) => (
        <div key={channel.id} className="flex flex-col border-b border-gray-300 py-2">
          <h2 className="">
            ▶️{channel.desc} - {channel.name}
          </h2>
          <p className="flex flex-row gap-4 text-sm text-gray-600">
            <Link to={`/channel/${channel.id}`} className="underline">
              Go to Channel Page
            </Link>
            <span>{channel.comment}</span>
            <span>{channel.genre}</span>
            <span>{channel.id}</span>
          </p>
        </div>
      ))}
    </MainLayout>
  )
}
