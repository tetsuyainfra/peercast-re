import { JsonChannel } from "@/_gen-api";
import ChannelLayout from "@/layouts/channelLayout";
import { useEffect } from "react";
import { useLoaderData, useOutletContext } from "react-router";


export default function Channel() {
  const { channel } = useLoaderData<{channel: JsonChannel}>();
  console.info("Channel page loader data:", channel);

  const pageTitle = `${channel.desc} - ${channel.name}`;

  return (
    <ChannelLayout pageTitle={pageTitle}>
      <div>{channel.desc} - {channel.name}</div>
    </ChannelLayout>
  )
}
