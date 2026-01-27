import * as gen from "@/_gen-api";
import { client } from "@/_gen-api/client.gen";
import { channel } from "diagnostics_channel";

// configure internal service client
client.setConfig({
  // set default base url for requests
  // baseUrl: "http://localhost:5173",
  baseUrl: "http://localhost:17145",
  // set default headers for requests
  headers: {
    // Access-Control-Allow-Headers: を明示しないといずれCORSエラーになる
    // Authorization: "Bearer <token_from_service_client>",
  },
})

export const channelApi = {
  show: (id: string) =>
    gen.showChannel({ path: { id } })
  ,
  list: (limit?: number) =>
    gen.listChannels()
};
