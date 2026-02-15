use bytes::BytesMut;
use tokio_util::codec::{Decoder, Encoder};

use crate::pcp::atom2::{atom_mut::AtomMut, parser::ParseError, Atom2};

pub struct AtomCodec;

impl AtomCodec {
    pub fn new() -> Self {
        Self {}
    }
}

impl Decoder for AtomCodec {
    type Item = Atom2;
    type Error = std::io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        match super::parser::try_parse_atom(src) {
            Ok(length) => {
                // MEMO: split_toでも大丈夫らしいけど本当に大丈夫なのかわからにゃい
                let atom_bytes = src.split_to(length);
                Ok(Some(Atom2::new(atom_bytes.freeze())))
            }
            Err(e) => match e {
                ParseError::UnexpectedEnd => Ok(None),
                ParseError::InvalidFormat => todo!(),
                ParseError::MalformedData => todo!(),
            },
        }
    }
}

impl Encoder<Atom2> for AtomCodec {
    type Error = std::io::Error;

    fn encode(&mut self, item: Atom2, dst: &mut BytesMut) -> Result<(), Self::Error> {
        // HACKME: 実装未完
        let x = item.write_buf(dst);
        Ok(())
    }
}

impl Encoder<AtomMut> for AtomCodec {
    type Error = std::io::Error;

    fn encode(&mut self, item: AtomMut, dst: &mut BytesMut) -> Result<(), Self::Error> {
        // HACKME: 実装未完
        let x = item.write(dst);
        Ok(())
    }
}

#[cfg(test)]
mod t {
    use crate::pcp::{atom2::AtomView, Id4};

    use super::*;
    use bytes::BufMut;
    use futures_util::{SinkExt, StreamExt};
    use tokio_util::codec::{Decoder, Encoder, Framed};

    #[ignore]
    #[test]
    fn test_atom_codec_decode() {
        let mut codec = AtomCodec::new();
        let mut buf = BytesMut::with_capacity(1024);

        // サンプルのAtomデータをバッファに追加
        buf.put(&b"abcd\x00\x00\x00\x00"[..]); // 8バイトのAtom

        // デコードを試みる
        match codec.decode(&mut buf) {
            Ok(Some(atom)) => {
                assert_eq!(atom.raw.len(), 8);
                assert_eq!(atom.id(), Id4::from(*b"abcd"));
                assert_eq!(atom.length(), 0);
                assert_eq!(atom.kind(), crate::pcp::atom2::Kind::Child);
            }
            Ok(None) => unreachable!(),
            Err(e) => unreachable!(),
        }
    }

    #[test]
    fn test_atom_codec_from_framed() {
        let (client, server) = tokio::io::duplex(1024);

        let mut framed_server = Framed::new(server, AtomCodec::new());
        let mut framed_client = Framed::new(client, AtomCodec::new());

        // client → server
        // framed_client.send(b"hello".to_vec()).await.unwrap();

        // let msg = framed_server.next().await.unwrap().unwrap();
        // assert_eq!(msg, b"hello");
    }

    // TODO: エンコードのテスト
    // 1. Atom2インスタンスをエンコードしてバイト列になること
    // 2. エンコードしたバイト列をデコードして元のAtom
    // TODO: 異常なデータが来た時のテスト
    // TODO: 大きなデータのテスト
    // TODO: 複数パケットの連続受信テスト
    // TODO: 部分パケット受信テスト
    // TODO: 境界値テスト
}
