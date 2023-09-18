import {  Configuration, ConfigApi } from "@peercast-client"
import React, { useEffect } from "react"

export default function Config() {
  let [config, setConfig] = React.useState("")
  useEffect(() => {
    ;(async () => {
      let api_config = new Configuration({ basePath: "http://localhost:17144/api" })
      let api = new ConfigApi(api_config)
      let config = await api.configGet()
      setConfig(config)
    })()
  }, [])
  return (
    <>
      <h1>Config Page</h1>
      <pre>{config}</pre>
    </>
  )
}
