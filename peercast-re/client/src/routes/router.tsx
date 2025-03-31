import Root from "../routes/root"
import Index from "../routes/_index"
import { createBrowserRouter } from "react-router-dom"
import ErrorPage from "@/pages/error-pages"
import Config from "@/pages/config"
import Channels from "@/pages/channels"

const router = createBrowserRouter(
  [
    {
      path: "/",
      element: <Root />,
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
      ],
    },
  ],
  { basename: "/ui" },
)

export default router
