import { JsonChannel } from "@/_gen-api";
import ChannelLayout, { OutletContext } from "@/layouts/channelLayout";
import { useEffect } from "react";
import { useLoaderData, useOutletContext } from "react-router";


export default function Channel() {
  const {pageTitle, setPageTitle} = useOutletContext<OutletContext>();

  const { channel } = useLoaderData<{channel: JsonChannel}>();
  console.info("Channel page loader data:", channel);

  // setPageTitle(channel.name);
  // useEffect(() => {
  //   setPageTitle(channel.name);
  // }, []);

  return (
    // <ChannelLayout pageTitle="a">
      <div>{channel.desc} - {channel.name}</div>
    // </ChannelLayout>
  )
}
