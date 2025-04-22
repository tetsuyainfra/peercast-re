import { Configuration } from "@peercast-api"

function addr_config(): [string, number] {
  let host = location.hostname
  let port = Number(location.port)
  if (import.meta.env.MODE == "development") {
    host = PEERCAST_HOST
    port = Number(PEERCAST_PORT)
  }

  return [host, port]
}

function api_config(): Configuration {
  let [host, port] = addr_config()
  let config = {
    basePath: `http://${host}:${port}/api`,
  }
  console.log("api_config:", config)
  return new Configuration(config)
}

function play_url(id: string): string {
  let [host, port] = addr_config()
  return `http://${host}:${port}/pls/${id}`
}

function stream_url(id: string): string {
  let [host, port] = addr_config()
  return `http://${host}:${port}/stream/${id}`
}

export { api_config, play_url }
