import { ScrollArea, ScrollBar } from "@/components/ui/scroll-area"
import { Separator } from "@/components/ui/separator"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import BrodcastChannelButton from "@/components/forms/BrodcastChannelForm"
import React, { useEffect, useState } from "react"
import ChannelCard from "@/components/ChannelCard"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { env } from "process"
// import { RespChannel } from "@re-api"

export default function Channels() {
  const urlInputId = React.useId()
  let [channels, channelsSet] = useState<any []>([])
  useEffect(() => {
    ;
    // (async () => {
    //   let api = new ChannelApi(api_config())
    //   await api.channelsGet().then(
    //     (channels) => {
    //       channelsSet(channels)
    //     },
    //     (err) => {
    //       console.info("channelsGet failed", err)
    //     },
    //   )
    // })()
  }, [])

  const addChannel = (evt: React.FormEvent<HTMLFormElement>) => {
    evt.preventDefault()
    // const form = evt.target as HTMLFormElement
    // const formData = new FormData(form)
    // console.log("evt: ", evt, formData.entries())
    let urlInput = (document.getElementById(urlInputId) as HTMLInputElement).value
    let url = new URL(urlInput)
    let id = url.pathname.split("/").at(-1) || ""
    let host = url.searchParams.get("tip") || ""

  // let api = new ChannelApi(api_config())
  //   api
  //     .createRelayChannel({
  //       reqCreateRelayChannel: {
  //         id: id,
  //         host: host,
  //       },
  //     })
  //     .then((channel) => {
  //       console.log("createRelayChannel", channel)
  //       window.location.reload()
  //     })
  }

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
          <div className="flex w-full max-w-sm">
            <form className="flex w-full max-w-sm items-center space-x-2" onSubmit={addChannel}>
              <Input
                type="url"
                id={urlInputId}
                name="source_url"
                defaultValue={import.meta.env.VITE_DEBUG_URL}
              />
              <Button type="submit">Play</Button>
            </form>
          </div>
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
