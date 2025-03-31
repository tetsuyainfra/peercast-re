import { zodResolver } from "@hookform/resolvers/zod"
import * as z from "zod"
import { useForm } from "react-hook-form"

import { PlusCircledIcon } from "@radix-ui/react-icons"

import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog"
import { Button } from "../ui/button"
import { Label } from "../ui/label"
import { Input } from "../ui/input"
import {
  Form,
  FormControl,
  FormDescription,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from "../ui/form"
import { SlimFormItem } from "../my-ui/slim-form"

import { Configuration, ChannelApi, ReqCreateChannel } from "@peercast-client"
import { hostname } from "os"
import { api_config } from "@/lib/api"
import React from "react"

const formSchema = z.object({
  name: z.string().min(1).max(64),
  genre: z.string().min(0).max(64),
})

export default function BrodcastChannelButton() {
  const [open, setOpen] = React.useState(false)

  const form = useForm<z.infer<typeof formSchema>>({
    resolver: zodResolver(formSchema),
    defaultValues: {
      name: "配信ちゃんねるTest",
      genre: "p@プログラミング",
    },
  })

  function onSubmit(values: z.infer<typeof formSchema>) {
    console.log("onSubmit", values)
    // Do something with the form values.
    // ✅ This will be type-safe and validated.
    // let api_config = new Configuration({ basePath: "http://localhost:17144/api" })
    let api = new ChannelApi(api_config())

    api
      .createBroadcastChannel({
        reqCreateChannel: {
          name: values.name,
          genre: values.genre,
        },
      })
      .then((resp) => {
        console.log("channelsPost: ", resp)
        setOpen(false)
        window.location.reload()
      })
  }

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>
        <Button>
          <PlusCircledIcon className="mr-2 h-4 w-4" />
          配信する
        </Button>
      </DialogTrigger>
      <DialogContent className="sm:max-w-[425px]">
        <DialogHeader>
          <DialogTitle>Edit Channel Information</DialogTitle>
          <DialogDescription>
            Make changes to your channel infomation here. Click broadcast channel ready when you're
            done.
          </DialogDescription>
        </DialogHeader>
        <Form {...form}>
          {/* <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-8"> */}
          <form
            onSubmit={form.handleSubmit(onSubmit)}
            // className="grid grid-cols-4 items-center gap-4"
          >
            <div className="grid gap-4 py-4">
              <FormField
                control={form.control}
                name="name"
                render={({ field }) => (
                  <SlimFormItem className="grid grid-cols-4 items-center gap-4">
                    <FormMessage className="col-span-4" />
                    <FormLabel className="text-right">Channel Name</FormLabel>
                    <FormControl className="col-span-3">
                      <Input placeholder="配信ちゃんねるch" {...field} />
                    </FormControl>
                    {/* <FormDescription>This is your public display name.</FormDescription> */}
                  </SlimFormItem>
                )}
              />

              <FormField
                control={form.control}
                name="genre"
                render={({ field }) => (
                  <SlimFormItem className="grid grid-cols-4 items-center gap-4">
                    <FormMessage className="col-span-4" />
                    <FormLabel className="text-right">Genre</FormLabel>
                    <FormControl className="col-span-3">
                      <Input placeholder="sp@Game" {...field} />
                    </FormControl>
                    {/* <FormDescription>This is your public display name.</FormDescription> */}
                  </SlimFormItem>
                )}
              />
            </div>

            <DialogFooter>
              <div className="flex-grow">
                <Button type="submit" disabled>
                  設定を保存する
                </Button>
              </div>
              <div>
                <Button type="submit" className="ml-auto">
                  配信開始
                </Button>
              </div>
            </DialogFooter>
          </form>
        </Form>
      </DialogContent>
    </Dialog>
  )
}

// export function ChannelForm() {
//   return <>Form</>
// }
