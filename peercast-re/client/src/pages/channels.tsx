import React, { useEffect, useState } from "react"
import { env } from "process"
// import { RespChannel } from "@re-api"

export default function Channels() {
  const urlInputId = React.useId()
  let [channels, channelsSet] = useState<any []>([])
  useEffect(() => {
    ;
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
    // const form = evt.target as HTMLFormElement
    // const formData = new FormData(form)
    // console.log("evt: ", evt, formData.entries())
    let urlInput = (document.getElementById(urlInputId) as HTMLInputElement).value
    let url = new URL(urlInput)
    let id = url.pathname.split("/").at(-1) || ""
    let host = url.searchParams.get("tip") || ""

  // let api = new ChannelApi(api_config())
  //   api
  //     .createRelayChannel({
  //       reqCreateRelayChannel: {
  //         id: id,
  //         host: host,
  //       },
  //     })
  //     .then((channel) => {
  //       console.log("createRelayChannel", channel)
  //       window.location.reload()
  //     })
  }

  return (
    <>
    </>
  )
}
