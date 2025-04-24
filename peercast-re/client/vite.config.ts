import path from "path"
import { defineConfig } from "vite"
import react from "@vitejs/plugin-react-swc"
import { env } from "process"

const PEERCAST_HOST = env.PEERCAST_RE_HOST || "localhost"
const PEERCAST_PORT = env.PEERCAST_RE_PORT || 17144

console.log(PEERCAST_HOST, PEERCAST_PORT)

// https://vitejs.dev/config/
export default defineConfig({
  base: "/ui",
  server: {
    hmr: {
      clientPort: 5173,
      host: "localhost",
    },

    // ブラウザからviteに直接アクセスした場合、本来のAPIへのプロキシが必要になる
    // っていうかいどうすればいいんだろ
    proxy: {
      "/api": {
        target: `http://${PEERCAST_HOST}:${PEERCAST_PORT}/`,
        // changeOrigin: true,
        // rewrite: (path) => path.replace(/^\/api/, ""),
      },
    },
  },
  plugins: [react()],
  resolve: {
    alias: {
      // "@peercast-api": path.resolve(__dirname, "../../libpeercast-re-apis/gen/ts-fetch"),
      "@re-api": path.resolve(__dirname, "./libapi"),
      "@": path.resolve(__dirname, "./src"),
    },
  },
  define: {
    PEERCAST_HOST: JSON.stringify(PEERCAST_HOST),
    PEERCAST_PORT: JSON.stringify(PEERCAST_PORT),
  },
})
