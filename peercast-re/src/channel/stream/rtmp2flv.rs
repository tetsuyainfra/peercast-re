use bytes::{BufMut, Bytes};
use rml_rtmp::{sessions::StreamMetadata, time::RtmpTimestamp};

pub struct Rtmp2Flv {
    metadata: StreamMetadata,
}

impl Rtmp2Flv {
    pub fn new(metadata: StreamMetadata) -> Self {
        Self {
            metadata,
        }
    }

    /// この関数はFLVヘッダーを生成し、Bytesとして返す
    /// また、一度この関数を呼び出した後は、ストリームの最初にFLVヘッダーを送信することを保証しなくてはならない
    pub fn header(&mut self) -> Bytes {
        let has_audio = self.metadata.audio_channels.is_some();
        let has_video = self.metadata.video_codec_id.is_some();
        let flv_header = FLVHeader::new(has_audio, has_video);
        flv_header.to_bytes()
    }

    pub fn push_video_data(&mut self, timestamp: RtmpTimestamp, data: bytes::Bytes) -> Bytes {
        FLVTag {
            filter: false,
            tag_type: TagType::Video,
            timestamp: timestamp.value,
            stream_id: 0,
            data,
        }
        .to_bytes()
    }
    pub fn push_audio_data(&mut self, timestamp: RtmpTimestamp, data: bytes::Bytes) -> Bytes {
        FLVTag {
            filter: false,
            tag_type: TagType::Audio,
            timestamp: timestamp.value,
            stream_id: 0,
            data,
        }
        .to_bytes()
    }
}

struct FLVHeader {
    pub version: u8,
    pub has_audio: bool,
    pub has_video: bool,
}

impl FLVHeader {
    pub fn new(has_audio: bool, has_video: bool) -> Self {
        Self {
            version: 1,
            has_audio,
            has_video,
        }
    }

    pub fn to_bytes(&self) -> bytes::Bytes {
        let mut buf = bytes::BytesMut::with_capacity(9 + 4);
        buf.extend_from_slice(b"FLV");
        buf.put_u8(self.version);
        let flags: u8 = (if self.has_audio {
            0b0000_0100
        } else {
            0
        }) | (if self.has_video {
            0b0000_0001
        } else {
            0
        });
        buf.put_u8(flags);
        buf.put_u32(9); // DataOffset
        buf.put_u32(0); // PreviousTagSize0
        buf.freeze()
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
enum TagType {
    Audio = 8,
    Video = 9,
    #[allow(dead_code)]
    ScriptData = 18,
}

struct FLVTag {
    // tag[0]
    pub filter: bool, // 0: normal, 1: needs decrypt
    pub tag_type: TagType,

    pub timestamp: u32, // 24bit + 8bit extended
    pub stream_id: u32, // always 0
    pub data: bytes::Bytes,
}

impl FLVTag {
    pub fn to_bytes(&self) -> bytes::Bytes {
        // TODO: check data size overflow
        let data_size = self.data.len() as u32;

        // TagHeader() + Data + PreviousTagSize(4bytes)
        let mut buf = bytes::BytesMut::with_capacity(11 + data_size as usize + 4);

        let first_byte = if self.filter {
            0b0100_0000 | (self.tag_type as u8)
        } else {
            self.tag_type as u8
        };
        buf.put_u8(first_byte);
        // data_size is 3 bytes
        buf.put_uint(data_size as u64, 3);
        // timestamp
        buf.put_uint((self.timestamp & 0x00FF_FFFF) as u64, 3);
        buf.put_u8(((self.timestamp & 0xFF00_0000) >> 24) as u8); // TimestampExtended
        // StreamID
        buf.put_uint(self.stream_id as u64, 3);
        // Data
        buf.extend_from_slice(&self.data);

        // PreviousTagSize (means of this tag)
        let previous_tag_size = 11 + data_size;
        buf.put_u32(previous_tag_size);
        buf.freeze()
    }
}

// RTMPからFLVへの変換には以下のヘッダを使わすとりあえずぶち込めばなんとかなるっぽい
/*
enum TagBody {
    Audio(AudioTag),
    Video(VideoTag),
    ScriptData(ScriptDataTag),
}
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
enum SoundFormat {
    LinearPCMPlatformEndian = 0,
    ADPCM = 1,
    MP3 = 2,
    LinearPCMLittleEndian = 3,
    Nellymoser16kHzMono = 4,
    Nellymoser8kHzMono = 5,
    Nellymoser = 6,
    G711ALawLogarithmicPCM = 7,
    G711MuLawLogarithmicPCM = 8,
    AAC = 10,
    Speex = 11,
    MP38kHzStereo = 14,
    DeviceSpecificSound = 15,
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
enum SoundRate {
    Hz5_5k = 0,
    Hz11k = 1,
    Hz22k = 2,
    Hz44k = 3,
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
enum SoundSize {
    Sound8Bit = 0,
    Sound16Bit = 1,
}
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
enum SoundType {
    Mono = 0,
    Stereo = 1,
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
enum AACPacketType {
    AACSequenceHeader = 0,
    AACRaw = 1,
}

struct AudioTag {
    data: bytes::Bytes,
}

impl AudioTag {
    fn to_bytes(&self) -> bytes::Bytes {
        let mut buf = bytes::BytesMut::with_capacity(1 + self.data.len());
        //
        buf.extend_from_slice(&self.data);
        buf.freeze()
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
enum FrameType {
    KeyFrame = 1,
    InterFrame = 2,
    DisposableInterFrame = 3,
    GeneratedKeyFrame = 4,
    VideoInfoFrame = 5,
}
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
enum CodecID {
    SorensonH263 = 2,
    ScreenVideo = 3,
    On2VP6 = 4,
    On2VP6WithAlphaChannel = 5,
    ScreenVideoVersion2 = 6,
    AVC = 7,
}
struct VideoTag {
    data: bytes::Bytes,
}

struct ScriptDataTag {}

struct Metadata {
    properties: HashMap<String, Amf0Value>,
}

impl From<StreamMetadata> for Metadata {
    fn from(metadata: StreamMetadata) -> Self {
        let mut properties = HashMap::with_capacity(11);
        metadata.video_width.map(|x| properties.insert("width".to_string(), Amf0Value::Number(x as f64)));

        metadata.video_height.map(|x| properties.insert("height".to_string(), Amf0Value::Number(x as f64)));

        metadata.video_codec_id.map(|x| properties.insert("videocodecid".to_string(), Amf0Value::Number(x as f64)));

        metadata
            .video_bitrate_kbps
            .map(|x| properties.insert("videodatarate".to_string(), Amf0Value::Number(x as f64)));

        metadata.video_frame_rate.map(|x| properties.insert("framerate".to_string(), Amf0Value::Number(x as f64)));

        metadata.audio_codec_id.map(|x| properties.insert("audiocodecid".to_string(), Amf0Value::Number(x as f64)));

        metadata
            .audio_bitrate_kbps
            .map(|x| properties.insert("audiodatarate".to_string(), Amf0Value::Number(x as f64)));

        metadata
            .audio_sample_rate
            .map(|x| properties.insert("audiosamplerate".to_string(), Amf0Value::Number(x as f64)));

        metadata.audio_channels.map(|x| properties.insert("audiochannels".to_string(), Amf0Value::Number(x as f64)));

        metadata.audio_is_stereo.map(|x| properties.insert("stereo".to_string(), Amf0Value::Boolean(x)));

        metadata.encoder.as_ref().map(|x| properties.insert("encoder".to_string(), Amf0Value::Utf8String(x.clone())));

        Self {
            properties,
        }
    }
}
    */

#[cfg(test)]
mod t {
    use crate::channel::stream::rtmp2flv::{FLVTag, TagType};

    #[test]
    fn test_flvtag() {
        let tag = FLVTag {
            filter: false,
            tag_type: TagType::Video,
            timestamp: 0,
            stream_id: 0,
            data: bytes::Bytes::from_static(b"\x17\x00\x00\x00\x00\x00\x00\x00\x00"),
        };

        let bytes = tag.to_bytes();
        assert_eq!(bytes.len(), 11 + 9 + 4);
        assert_eq!(bytes[0], 9); // tag_type
        assert_eq!(bytes[1], 0); // data_size
        assert_eq!(bytes[2], 0);
        assert_eq!(bytes[3], 9);
        assert_eq!(bytes[4], 0); // timestamp
        assert_eq!(bytes[5], 0);
        assert_eq!(bytes[6], 0);
        assert_eq!(bytes[7], 0); // timestamp extended
        assert_eq!(bytes[8], 0); // stream_id
        assert_eq!(bytes[9], 0);
        assert_eq!(bytes[10], 0);
        // data
        assert_eq!(bytes[11], 0x17);
        assert_eq!(bytes[12], 0x00);
        assert_eq!(bytes[19], 0x00);
        // previous tag size
        assert_eq!(bytes[20], 0);
        assert_eq!(bytes[21], 0);
        assert_eq!(bytes[22], 0);
        assert_eq!(bytes[23], 20); // 11 + 9
    }
}

// FLV format reference: ADOBE FLASH VIDEO FILE FORMAT SPECIFICATION VERSION 10.1 68
// https://ossrs.net/lts/zh-cn/assets/files/flv_v10_1-5244d3059e5425eb03a15eb24537bd59.pdf
// https://veovera.org/docs/legacy/video-file-format-v10-1-spec.pdf
// https://github.com/monyone/mikan/blob/main/core/src/chunk/flv.ts

/*
# FLV ファイル構造
FLVファイル = FLVHeader + FLVBody
FLVBody = PreviousTagSize0 + Tag1 + PreviousTagSize1 + Tag2 + ... + PreviousTagSizeN-1 + TagN
↓次のように読み替えても良い
FLVファイル = FLVHeader + PreviousTagSize0 + FLVBody
FLVBody = Tag1 + PreviousTagSize1 + Tag2 + ... + PreviousTagSizeN-1 + TagN

# FLV Header

# FLV Body
FLV Body は複数の Tag で構成される
Tag = TagHeader + Data
PreviousTagSize = UI32


*/
