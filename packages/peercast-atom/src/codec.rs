use bytes::BytesMut;
use tokio_util::codec::{Decoder, Encoder};

use crate::{
    Atom, AtomMut,
    error::{AtomCodecError, AtomParseError},
    parser::AtomParser,
};

#[derive(Debug)]
pub struct AtomCodec {
    parser: AtomParser,
}

impl AtomCodec {
    pub fn new(max_atom_size: u32, max_children_num: u32) -> Self {
        Self {
            parser: AtomParser::new(max_atom_size, max_children_num),
        }
    }
}

impl Default for AtomCodec {
    fn default() -> Self {
        Self {
            parser: AtomParser::default(),
        }
    }
}

impl Decoder for AtomCodec {
    type Item = Atom;
    type Error = AtomCodecError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        match self.parser.try_parse(src) {
            Ok(length) => {
                // MEMO: split_toでも大丈夫らしいけど本当に大丈夫なのかわからにゃい
                let atom_bytes = src.split_to(length);
                Ok(Some(Atom::new(atom_bytes.freeze())))
            }
            Err(AtomParseError::UnexpectedEof) => {
                // データ着信途中の可能性があるので、Noneで返す
                Ok(None)
            }
            Err(e) => Err(e.into()),
        }
    }
}

impl Encoder<Atom> for AtomCodec {
    type Error = AtomCodecError;

    fn encode(&mut self, item: Atom, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let _ = item.write_to(dst);
        Ok(())
    }
}

impl Encoder<AtomMut> for AtomCodec {
    type Error = AtomCodecError;

    fn encode(&mut self, item: AtomMut, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let _ = item.write_to(dst)?;
        Ok(())
    }
}

#[cfg(test)]
mod t {
    use crate::AtomView;

    use super::*;
    use bytes::BufMut;
    use tokio_util::codec::Framed;

    #[ignore]
    #[test]
    fn test_atom_codec_decode() {
        let mut codec = AtomCodec::default();
        let mut buf = BytesMut::with_capacity(1024);

        // サンプルのAtomデータをバッファに追加
        buf.put(&b"abcd\x00\x00\x00\x00"[..]); // 8バイトのAtom

        // デコードを試みる
        match codec.decode(&mut buf) {
            Ok(Some(atom)) => {
                assert_eq!(atom.payload().len(), 8);
                // assert_eq!(atom.id(), Id4::from(*b"abcd"));
                assert_eq!(atom.length(), 0);
                assert_eq!(atom.kind(), crate::AtomKind::Child);
            }
            Ok(None) => unreachable!(),
            Err(_e) => unreachable!(),
        }
    }

    #[test]
    fn test_atom_codec_from_framed() {
        let (client, server) = tokio::io::duplex(1024);

        let _framed_server = Framed::new(server, AtomCodec::default());
        let _framed_client = Framed::new(client, AtomCodec::default());

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
