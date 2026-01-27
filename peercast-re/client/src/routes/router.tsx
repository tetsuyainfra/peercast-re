// import Index from "../routes/_index"
import { createBrowserRouter, redirect } from "react-router"

import Root from "../routes/root"
import ErrorPage from "@/pages/error-pages"
import Index from "@/pages/index"
import Config from "@/pages/config"
import Channels from "@/pages/channels"
import Channel from "@/pages/channel"
import YellowPages from "@/pages/yellowPages"
import ChannelLayout from "@/layouts/channelLayout"
import { channelApi } from "@/api"

const router = createBrowserRouter(
  [
    // Root Object
    {
      path: "/",
      errorElement: <ErrorPage />,
      children: [
        {
          index: true,
          element: <Index />,
        },
      ],
    },
    {
      path: "/config",
      element: <Config />,
    },
    {
      path: "/channels",
      element: <Channels />,
    },
    {
      path: "/channel",
      Component: ChannelLayout,
      children: [
        {
          path: ":channelId",
          Component: Channel,
          HydrateFallback: () => <div>Loading...</div>,
          loader: async ({ params }) => {
            console.info("channel loader", { params })
            let { data, error } = await channelApi.show(params.channelId!)
            if (error) {
              console.info("redirect to /channels")
              throw redirect("/channels")
            }
            return data
          },
        },
      ],
    },
    {
      path: "/yellowpages",
      element: <YellowPages />,
    },
  ],
  { basename: "/ui" },
)

export default router
