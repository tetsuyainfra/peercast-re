import Root from "../routes/root"
// import Index from "../routes/_index"
import { createBrowserRouter, redirect } from "react-router"

import Root from "../routes/root"
import ErrorPage from "@/pages/error-pages"
import Index from "@/pages/index"
import Config from "@/pages/config"
import Channels from "@/pages/channels"
import Channel from "@/pages/channel"
import YellowPages from "@/pages/yellowPages"

const router = createBrowserRouter(
  [
    {
      path: "/",
      // element: <Root />,
      errorElement: <ErrorPage />,
      children: [
        {
          index: true,
          element: <Index />,
        },
        {
          path: "config/",
          element: <Config />,
        },
        {
          path: "channels/",
          element: <Channels />,
        },
        {
          path: "channel/",
          element: <Channel />,
        },
        {
          path: "yellowpages/",
          element: <YellowPages />,
        },
      ],
    },
  ],
  { basename: "/ui" },
)

export default router
