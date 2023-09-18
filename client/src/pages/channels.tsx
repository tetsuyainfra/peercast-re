import { ScrollArea, ScrollBar } from "@/components/ui/scroll-area"
import { Separator } from "@/components/ui/separator"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import BrodcastChannelButton from "@/components/forms/BrodcastChannelForm"
import { useEffect, useState } from "react"
import { api_config } from "@/lib/api"
import { ChannelApi } from "../../../gen/ts-fetch/apis"
import { RespChannel } from "../../../gen/ts-fetch/models"
import ChannelCard from "@/components/ChannelCard"

export default function Channels() {
  let [channels, channelsSet] = useState<RespChannel[]>([])
  useEffect(() => {
    ;(async () => {
      let api = new ChannelApi(api_config())
      let channels = await api.channelsGet()
      // console.log("channelsGet", channels)
      channelsSet(channels)
    })()
  }, [])
  return (
    <>
      <Tabs defaultValue="all" className="h-full space-y-6">
        <div className="space-between flex items-center">
          <TabsList>
            <TabsTrigger value="all" className="">
              All
            </TabsTrigger>
            <TabsTrigger value="mylive" className="" disabled>
              My Live
            </TabsTrigger>
            <TabsTrigger value="streaming" className="" disabled>
              Streaming
            </TabsTrigger>
            <TabsTrigger value="idle" className="" disabled>
              Idle
            </TabsTrigger>
          </TabsList>
          <div className="ml-auto mr-4">
            <BrodcastChannelButton />
          </div>
        </div>
        {/*  All */}
        <TabsContent value="all" className="border-none p-0 outline-none">
          <ul>
            {channels.map((ch, i) => {
              return <ChannelCard key={ch.id} channel={ch} />
            })}
            {/* {channels.map((ch, i) => {
              return (
                <li key={i}>
                  <span>{ch.info.name}</span>
                  <br />
                  <span>
                    [{ch.info.genre} - {ch.info.desc}]
                  </span>
                  <span>{ch.info.comment}</span>
                </li>
              )
            })} */}
          </ul>
        </TabsContent>
        {/*  mylive */}
        <TabsContent value="mylive" className="border-none p-0 outline-none">
          <div className="flex items-center justify-between">
            <div className="space-y-1">
              <h2 className="text-2xl font-semibold tracking-tight">Listen Now</h2>
              <p className="text-sm text-muted-foreground">Top picks for you. Updated daily.</p>
            </div>
          </div>
          <Separator className="my-4" />
        </TabsContent>
        {/*  Streaming */}
        <TabsContent value="streaming" className="border-none p-0 outline-none">
          <div className="flex items-center justify-between">
            <div className="space-y-1">
              <h2 className="text-2xl font-semibold tracking-tight">Listen Now</h2>
              <p className="text-sm text-muted-foreground">Top picks for you. Updated daily.</p>
            </div>
          </div>
          <Separator className="my-4" />
          <div className="relative">
            <ScrollArea>
              <div className="flex space-x-4 pb-4">
                {/* {listenNowAlbums.map((album) => (
                  <AlbumArtwork
                    key={album.name}
                    album={album}
                    className="w-[250px]"
                    aspectRatio="portrait"
                    width={250}
                    height={330}
                  />
                ))} */}
              </div>
              <ScrollBar orientation="horizontal" />
            </ScrollArea>
          </div>
          <div className="mt-6 space-y-1">
            <h2 className="text-2xl font-semibold tracking-tight">Made for You</h2>
            <p className="text-sm text-muted-foreground">Your personal playlists. Updated daily.</p>
          </div>
          <Separator className="my-4" />
          <div className="relative">
            <ScrollArea>
              <div className="flex space-x-4 pb-4">
                {/* {madeForYouAlbums.map((album) => (
                  <AlbumArtwork
                    key={album.name}
                    album={album}
                    className="w-[150px]"
                    aspectRatio="square"
                    width={150}
                    height={150}
                  />
                ))} */}
              </div>
              <ScrollBar orientation="horizontal" />
            </ScrollArea>
          </div>
        </TabsContent>
        {/* Idle */}
        <TabsContent
          value="idle"
          // className="h-full flex-col border-none p-0 data-[state=active]:flex"
          className="h-full flex-col border-none p-0"
        >
          <div className="flex items-center justify-between">
            <div className="space-y-1">
              <h2 className="text-2xl font-semibold tracking-tight">New Episodes</h2>
              <p className="text-sm text-muted-foreground">
                Your favorite podcasts. Updated daily.
              </p>
            </div>
          </div>
          <Separator className="my-4" />
          {/* <PodcastEmptyPlaceholder /> */}
        </TabsContent>
      </Tabs>
    </>
  )
}
