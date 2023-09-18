/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_APP_TITLE: string
  // readonly PEERCAST_RE_HOST: string
  // readonly PEERCAST_RE_PORT: number
  // その他の環境変数...
}

interface ImportMeta {
  readonly env: ImportMetaEnv
}

declare const PEERCAST_HOST: string
declare const PEERCAST_PORT: string
