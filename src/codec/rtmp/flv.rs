use std::{collections::HashMap, default, io};

use bitflags::bitflags;
use bytes::Bytes;
use flavors::parser::{Header, TagHeader};
use futures_util::Stream;
use rml_amf0::Amf0Value;
use rml_rtmp::sessions::StreamMetadata;
use tokio::io::{AsyncWrite, AsyncWriteExt};
use tracing::debug;
use tracing_subscriber::registry::Data;
use uuid::timestamp;

pub struct FlvWriter<W> {
    writer: W,
    last_stream_id_tag_length: HashMap<u32, u32>, // (stream_id, length)
}

impl<W> FlvWriter<W>
where
    W: AsyncWrite + AsyncWriteExt + Unpin,
{
    pub fn new(writer: W) -> Self {
        Self {
            writer,
            last_stream_id_tag_length: Default::default(),
        }
    }

    #[rustfmt::skip]
    pub async fn write_magic(&mut self, header: Header) -> Result<(), io::Error> {
        self.writer.write_all(b"FLV").await?;
        self.writer.write_u8(header.version).await?;
        let mut flags = 0_u8;
        if header.video { flags |= 0b0000_0001 };
        if header.audio { flags |= 0b0000_0100 };
        self.writer.write_u8(flags).await?;
        self.writer.write_u32(header.offset).await
        // self.writer.write_u32(0_u32).await // write empty previous tag
    }

    pub async fn write_header(
        &mut self,
        stream_id: u32,
        metadata: StreamMetadata,
    ) -> Result<(), io::Error> {
        // self.write_magic(Header).await?;
        // let mut meta = Metadata::from(metadata);
        // let Metadata { properties } = meta;

        // let message = vec![
        //     Amf0Value::Utf8String("onMetaData".to_string()),
        //     Amf0Value::Object(properties),
        // ];

        let audio = metadata.audio_codec_id.is_some();
        let video = metadata.video_codec_id.is_some();
        self.write_magic(Header {
            version: 1,
            audio,
            video,
            offset: 9,
        })
        .await
    }

    pub async fn write_video(&mut self, stream_id: u32, timestamp: u32, data: Bytes) {
        let last_tag_length = self.last_stream_id_tag_length.get(&stream_id);
        let last_tag_length = last_tag_length.map_or_else(|| 0, |v| *v);
        let payloda_size = data.len();

        TaggedData {
            last_tag_length,
            data_type: DataType::VIDEO,
            timestamp,
            stream_id,
            data,
        }
        .write(&mut self.writer)
        .await;

        self.last_stream_id_tag_length
            .insert(stream_id, (payloda_size + 11) as u32);
    }

    pub async fn write_audio(&mut self, stream_id: u32, timestamp: u32, data: Bytes) {
        let last_tag_length = self.last_stream_id_tag_length.get(&stream_id);
        let last_tag_length = last_tag_length.map_or_else(|| 0, |v| *v);
        let payloda_size = data.len();

        TaggedData {
            last_tag_length,
            data_type: DataType::AUDIO,
            timestamp,
            stream_id,
            data,
        }
        .write(&mut self.writer)
        .await;

        self.last_stream_id_tag_length
            .insert(stream_id, (payloda_size + 11) as u32);
    }
}

pub struct DataType(pub u8);
impl DataType {
    pub const AUDIO: Self = Self(0x08);
    pub const VIDEO: Self = Self(0x09);
    pub const SCRIPT: Self = Self(0x12);
}

pub struct TaggedData {
    last_tag_length: u32,
    data_type: DataType,
    timestamp: u32, // LSB24bit, MSB8bit
    stream_id: u32, // 24bit
    data: Bytes,
}
impl TaggedData {
    pub async fn write<W: AsyncWrite + Unpin>(&self, writer: &mut W) {
        // debug!(
        //     "write({} {} {} {})",
        //     self.data_type.0,
        //     self.data.len(),
        //     self.timestamp,
        //     self.stream_id
        // );
        let last_size = self.last_tag_length.to_be_bytes();
        let tag_type = self.data_type.0;
        let data_size = (self.data.len() as u32).to_be_bytes();
        let timestamps = self.timestamp.to_be_bytes();
        let stream_ids = self.stream_id.to_be_bytes();
        // debug!(?data_size);
        // debug!(?timestamps);
        // debug!(?stream_ids);

        let buf = [
            // last_tag_size
            last_size[0],
            last_size[1],
            last_size[2],
            last_size[3],
            // tag type
            tag_type,
            // datasize (24bit)
            data_size[1],
            data_size[2],
            data_size[3],
            // timestamp (LSB 24bit)
            timestamps[1],
            timestamps[2],
            timestamps[3],
            // timestamp extend (MSB 8bit)
            timestamps[0],
            // stream_id 24bit
            stream_ids[1],
            stream_ids[2],
            stream_ids[3],
        ];

        // write header
        writer.write(&buf).await;
        // payload
        writer.write(&self.data).await;
    }
}

//
struct Metadata {
    pub properties: HashMap<String, Amf0Value>,
}

impl From<StreamMetadata> for Metadata {
    fn from(metadata: StreamMetadata) -> Self {
        let mut properties = HashMap::with_capacity(11);
        metadata
            .video_width
            .map(|x| properties.insert("width".to_string(), Amf0Value::Number(x as f64)));

        metadata
            .video_height
            .map(|x| properties.insert("height".to_string(), Amf0Value::Number(x as f64)));

        metadata
            .video_codec_id
            .map(|x| properties.insert("videocodecid".to_string(), Amf0Value::Number(x as f64)));

        metadata
            .video_bitrate_kbps
            .map(|x| properties.insert("videodatarate".to_string(), Amf0Value::Number(x as f64)));

        metadata
            .video_frame_rate
            .map(|x| properties.insert("framerate".to_string(), Amf0Value::Number(x as f64)));

        metadata
            .audio_codec_id
            .map(|x| properties.insert("audiocodecid".to_string(), Amf0Value::Number(x as f64)));

        metadata
            .audio_bitrate_kbps
            .map(|x| properties.insert("audiodatarate".to_string(), Amf0Value::Number(x as f64)));

        metadata
            .audio_sample_rate
            .map(|x| properties.insert("audiosamplerate".to_string(), Amf0Value::Number(x as f64)));

        metadata
            .audio_channels
            .map(|x| properties.insert("audiochannels".to_string(), Amf0Value::Number(x as f64)));

        metadata
            .audio_is_stereo
            .map(|x| properties.insert("stereo".to_string(), Amf0Value::Boolean(x)));

        metadata
            .encoder
            .as_ref()
            .map(|x| properties.insert("encoder".to_string(), Amf0Value::Utf8String(x.clone())));

        Self { properties }
    }
}

#[cfg(test)]
mod test_spec {
    use super::*;
    use rml_rtmp::sessions::StreamMetadata;

    const SAMPLE: &[u8] = include_bytes!("../../../tmp/output.mp4");

    #[test]
    fn test_bitshift() {
        let v = 0x11_22_33_44_u32;
        let r = (v & 0xFF);
        assert_eq!(0x44, r);
        let r = ((v >> 8) & 0xFF) as u8;
        assert_eq!(0x33, r);
        let r = ((v >> 16) & 0xFF) as u8;
        assert_eq!(0x22, r);
        {
            let r = ((v >> 24) & 0xFF) as u8;
            assert_eq!(0x11, r);
            let r = (v >> 24) as u8;
            assert_eq!(0x11, r);
        }
    }

    #[crate::test]
    async fn test_writer() {
        let writer = tokio::fs::File::create("./tmp/test.flv").await.unwrap();
        let mut flv = FlvWriter::new(writer);
        // let meta = StreamMetadata {
        //     video_width: todo!(),
        //     video_height: todo!(),
        //     video_codec_id: todo!(),
        //     video_frame_rate: todo!(),
        //     video_bitrate_kbps: todo!(),
        //     audio_codec_id: todo!(),
        //     audio_bitrate_kbps: todo!(),
        //     audio_sample_rate: todo!(),
        //     audio_channels: todo!(),
        //     audio_is_stereo: todo!(),
        //     encoder: todo!(),
        // };
        // flv.write_metadata(meta).await
    }
}
