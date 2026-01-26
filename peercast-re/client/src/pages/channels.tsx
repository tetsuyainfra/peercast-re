import React, { useEffect, useState } from "react"
import { env } from "process"
import MainLayout from "@/layouts/mainLayout"
// import { RespChannel } from "@re-api"
import { client } from "@/api/client.gen"
import { createClient } from "@/api/client"
import { listChannels } from "@/api"

// configure internal service client
client.setConfig({
  // set default base url for requests
  baseUrl: "http://localhost:5173",
  // set default headers for requests
  headers: {
    Authorization: "Bearer <token_from_service_client>",
  },
})

export default function Channels() {
  const urlInputId = React.useId()
  let [channels, channelsSet] = useState<any[]>([])
  useEffect(() => {
    (async () => {
      const { data, error } = await listChannels();
      console.info("listChannels", { data, error });
      channelsSet(data as any || []);
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

  return <MainLayout pageTitle="Channels">
    {channels.map((channel) => (
      <div key={channel.id}>
        <h2>{channel.name} ({channel.id})</h2>
        <p>{channel.description}</p>
      </div>
    ))}
    </MainLayout>
}
