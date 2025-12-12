// import {  Configuration, ConfigApi } from "@peercast-api"
import React, { useEffect } from "react"
import MainLayout from "@/layouts/mainLayout";

export default function Config() {
  let [config, setConfig] = React.useState("")
  useEffect(() => {
    ;(async () => {
      // let api_config = new Configuration({ basePath: "http://localhost:17144/api" })
      // let api = new ConfigApi(api_config)
      // let config = await api.configGet()
      setConfig(config)
    })()
  }, [])
  return (
    <MainLayout pageTitle="Configuration">
      {config}
    </MainLayout>
  )
}



