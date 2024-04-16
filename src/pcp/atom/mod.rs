pub mod decode;
pub mod encode;

use core::fmt;
use std::{
    fmt::{format, Debug},
    io,
    net::IpAddr,
    sync::Arc,
};

use bytes::{Buf, BufMut, Bytes, BytesMut};
use clap::builder::UnknownArgumentValueParser;
use num::traits::ToBytes;
use once_cell::sync::Lazy;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tracing::{info, trace};

use crate::error::AtomParseError;

use super::{gnuid::GnuId, util::enable_msb_1, Id4};

/// Atomは常に完全な状態のパケットを示している。
/// 常に完全な状態であるということで。どのメソッドを読んでも壊れていることはない
#[derive(Debug, Clone, PartialEq)]
pub enum Atom {
    Parent(ParentAtom),
    Child(ChildAtom),
}

impl Atom {
    pub fn id(&self) -> Id4 {
        match self {
            Atom::Parent(p) => p.id(),
            Atom::Child(c) => c.id(),
        }
    }
    pub fn len(&self) -> u32 {
        match self {
            Atom::Parent(p) => p.len(),
            Atom::Child(c) => c.len(),
        }
    }
    pub fn is_parent(&self) -> bool {
        match self {
            Atom::Parent(_) => true,
            Atom::Child(_) => false,
        }
    }
    pub fn is_child(&self) -> bool {
        match self {
            Atom::Parent(_) => false,
            Atom::Child(_) => true,
        }
    }

    pub fn as_parent(&self) -> &ParentAtom {
        match self {
            Atom::Parent(p) => p,
            Atom::Child(_) => panic!("this atom is not parent"),
        }
    }
    pub fn as_child(&self) -> &ChildAtom {
        match self {
            Atom::Parent(_) => panic!("this atom is not child"),
            Atom::Child(c) => c,
        }
    }

    // Atomにした時のバイトサイズ
    pub fn atom_bytes(&self) -> usize {
        match self {
            Atom::Parent(p) => p.atom_bytes(),
            Atom::Child(c) => c.atom_bytes(),
        }
    }

    pub fn write_bytes(&self, buf: &mut BytesMut) {
        match self {
            Atom::Parent(p) => p.write_bytes(buf),
            Atom::Child(c) => c.write_bytes(buf),
        }
    }
    pub async fn write_stream<T>(&self, stream: T) -> Result<(), std::io::Error>
    where
        T: AsyncWrite + Unpin,
    {
        match self {
            Atom::Parent(p) => p.write_stream(stream).await,
            Atom::Child(c) => c.write_stream(stream).await,
        }
    }

    pub fn parse(buf: &mut BytesMut) -> Result<Atom, AtomParseError> {
        _internal::parse(buf)
    }
    pub(super) fn unchecked_parse(buf: &mut Bytes) -> Atom {
        _internal::unchecked_parse(buf)
    }

    pub fn parseable(buf: &[u8]) -> Result<u32, AtomParseError> {
        // u32は危ないような危なくないような・・・
        _internal::parseable(&buf)
    }
}

impl Atom {
    pub const AGENT: Lazy<Atom> =
        Lazy::new(|| Atom::Child((Id4::PCP_HELO_AGENT, crate::PKG_AGENT.clone() + "\0").into()));
    pub const VERSION: Lazy<Atom> =
        Lazy::new(|| Atom::Child((Id4::PCP_HELO_VERSION, 1218_u32).into()));
}
impl fmt::Display for Atom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Atom::Child(a) => write!(f, "Atom::Child({:?} [{}])", a.id(), a.len()),
            Atom::Parent(a) => write!(f, "Atom::Parent({:?} [{}])", a.id(), a.len()),
        }
    }
}

//--------------------------------------------------------------------------------
// ParentAtomの定義
//

#[derive(Debug, Clone, PartialEq)]
pub struct ParentAtom {
    id: Id4,
    childs: Vec<Atom>,
}

impl ParentAtom {
    pub fn new(id: Id4, childs: Vec<Atom>) -> ParentAtom {
        ParentAtom { id, childs }
    }

    pub fn id(&self) -> Id4 {
        self.id
    }
    pub fn len(&self) -> u32 {
        self.childs.len() as u32
    }
    pub fn childs(&self) -> &Vec<Atom> {
        self.childs.as_ref()
    }

    pub fn raw_parts(self) -> (Id4, Vec<Atom>) {
        (self.id, self.childs)
    }

    pub fn atom_bytes(&self) -> usize {
        self.childs.iter().fold(8, |sum, a| sum + a.atom_bytes())
    }

    pub fn write_bytes(&self, buf: &mut BytesMut) {
        buf.put_u32(self.id.0);
        buf.put_u32_le(enable_msb_1(self.childs.len() as u32));
        for atom in self.childs() {
            atom.write_bytes(buf)
        }
    }
    pub async fn write_stream<T>(&self, mut stream: T) -> Result<(), std::io::Error>
    where
        T: AsyncWrite + Unpin,
    {
        // header
        let mut buf = BytesMut::with_capacity(self.atom_bytes());
        self.write_bytes(&mut buf);

        stream.write_all(&buf).await
    }
}

//--------------------------------------------------------------------------------
// ChildAtomの定義
//
#[derive(Clone, PartialEq)]
pub struct ChildAtom {
    head_and_payload: Bytes,
}
impl ChildAtom {
    pub fn new(id: Id4, payload: &Bytes) -> ChildAtom {
        let length = payload.len();
        debug_assert!(
            length <= (i32::MAX as usize),
            "payload length must be less than i32::MAX"
        ); // MSBはparentに利用されるのでi32のMAXが実際の所の最大値

        let mut b = BytesMut::with_capacity(length + 8);
        b.put_u32(id.into()); // BE
        b.put_u32_le(length as u32); // LE
        b.put_slice(&payload[..]);
        ChildAtom {
            head_and_payload: b.freeze(),
        }
    }

    /// Bytes構造体からAtomを作る
    pub fn new_from_bytes(head_and_payload: Bytes) -> Self {
        Self { head_and_payload }
    }

    pub fn id(&self) -> Id4 {
        let x: &[u8; 4] = &self.head_and_payload[0..4].try_into().unwrap();
        Id4::from(*x)
    }
    pub fn len(&self) -> u32 {
        (self.head_and_payload.len() - 8) as u32
    }
    pub fn payload(&self) -> Bytes {
        self.head_and_payload.slice(8..)
    }

    pub fn atom_bytes(&self) -> usize {
        self.head_and_payload.len()
    }

    pub fn split_parts(mut self) -> (Id4, Bytes) {
        let id: Id4 = Id4::from(self.head_and_payload.get_u32());
        let _len = Id4::from(self.head_and_payload.get_u32_le());
        (id, self.head_and_payload)
    }

    pub fn raw_parts(self) -> (Bytes,) {
        (self.head_and_payload,)
    }

    pub fn write_bytes(&self, buf: &mut BytesMut) {
        buf.put(&self.head_and_payload[..]);
    }

    pub async fn write_stream<T>(&self, mut stream: T) -> Result<(), std::io::Error>
    where
        T: AsyncWrite + Unpin,
    {
        stream.write_all(&self.head_and_payload[..]).await
    }
}

impl Debug for ChildAtom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ChildAtom")
            // .field("head_and_payload", &self.head_and_payload)
            .field("head_and_payload", &ShortPrintBytes(&self.head_and_payload))
            .finish()
    }
}

//--------------------------------------------------------------------------------
//  DebugPrint for Bytes
//
struct ShortPrintBytes<'a>(&'a Bytes);
impl Debug for ShortPrintBytes<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fn w(b: u8, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            // https://doc.rust-lang.org/reference/tokens.html#byte-escapes
            if b == b'\n' {
                write!(f, "\\n")?;
            } else if b == b'\r' {
                write!(f, "\\r")?;
            } else if b == b'\t' {
                write!(f, "\\t")?;
            } else if b == b'\\' || b == b'"' {
                write!(f, "\\{}", b as char)?;
            } else if b == b'\0' {
                write!(f, "\\0")?;
            // ASCII printable
            } else if (0x20..0x7f).contains(&b) {
                write!(f, "{}", b as char)?;
            } else {
                write!(f, "\\x{:02x}", b)?;
            }
            Ok(())
        }

        let len = self.0.len();
        if len >= 25 {
            write!(f, "b\"")?;
            for &b in &self.0[0..12] {
                w(b, f)?;
                // write!(f, "{:02x} ", b);  16進数表記
            }
            write!(f, " .. ")?;
            for &b in &self.0[(len - 8)..len] {
                w(b, f)?;
                // write!(f, "{:02x} ", b);  16進数表記
            }
            write!(f, "\"")?;
            write!(f, "[{}]", len)?;
        } else {
            write!(f, "b\"")?;
            for &b in self.0 {
                w(b, f)?;
            }
            write!(f, "\"")?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod t {
    use std::{collections::VecDeque, hash::BuildHasher, io::BufWriter};
    use tokio_test::io::Builder as MockBuilder;

    use crate::show_size;

    use super::*;

    #[test]
    fn test_atoms() {
        let c = ChildAtom::from((Id4::PCP_OK, 1_u8));
        let a = Atom::Child(c.clone());
        assert_eq!(a.is_child(), true);
        assert_eq!(a.is_parent(), false);
        assert_eq!(a.len(), 1);
        let Atom::Child(c) = a else { panic!() };
        let (id, mut b) = c.split_parts();
        assert_eq!(id, Id4::PCP_OK);
        assert_eq!(b.len(), 1);
        assert_eq!(b.get_u8(), 1_u8);

        let c = ChildAtom::from((Id4::PCP_OLEH, 2_u32));
        let a = Atom::Child(c.clone());
        assert_eq!(a.len(), 4);
        let Atom::Child(c) = a else { panic!() };
        let (id, mut b) = c.split_parts();
        assert_eq!(id, Id4::PCP_OLEH);
        assert_eq!(b.len(), 4);
        assert_eq!(b.get_u32_le(), 2_u32);

        let gid = GnuId::new();
        let c = ChildAtom::from((Id4::PCP_OK, gid.clone()));
        let a = Atom::Child(c.clone());
        assert_eq!(a.len(), 16);
        let Atom::Child(c) = a else { panic!() };
        let (id, mut b) = c.split_parts();
        assert_eq!(id, Id4::PCP_OK);
        assert_eq!(b.len(), 16);
        assert_eq!(b.get_u128(), gid.0);

        let c = ChildAtom::from((Id4::PCP_OK, gid.clone()));
        let a = Atom::Child(c.clone());
        let a = Atom::Parent(ParentAtom {
            id: Id4::PCP_CHAN,
            childs: vec![a.clone(), a.clone()],
        });
        assert_eq!(a.is_parent(), true);
        assert_eq!(a.is_child(), false);
        assert_eq!(a.len(), 2);

        // TODO: write_to_bufferのテスト書く
    }

    #[crate::test]
    async fn test_parent_atoms() {
        let patom = ParentAtom::from((Id4::PCP_ROOT_UPDATE, vec![]));
        assert_eq!(8, patom.atom_bytes());

        let mut mock = MockBuilder::new()
            .write(&Id4::PCP_ROOT_UPDATE.0.to_be_bytes())
            .write(&enable_msb_1(0).to_le_bytes())
            .build();

        let _ = patom.write_stream(&mut mock).await;
    }

    #[crate::test]
    async fn atom_values() {
        let atom = ChildAtom::from((
            Id4::PCP_HELO_REMOTEIP,
            "192.168.10.1".parse::<IpAddr>().unwrap(),
        ));

        // - IP 192.168.10.1 -> payload : 0x01_0A_A8_C0 / 01=1, 0A=10, A8=168, C0=192  IPはLEで格納されている
        let mut mock = MockBuilder::new()
            .write(&Id4::PCP_HELO_REMOTEIP.0.to_be_bytes())
            .write(&(4_u32.to_le_bytes()))
            .write(&(0xC0_A8_0A_01_u32.to_le_bytes()))
            .build();
        let _ = atom.write_stream(&mut mock).await;
    }

    #[test]
    fn test_child_atoms() {}

    #[ignore = "this is spec check for dev"]
    #[allow(dead_code)]
    #[test]
    fn test_atoms_specs() {
        show_size!(Bytes);
        show_size!(Atom);
        show_size!(ParentAtom);
        show_size!(ChildAtom);

        println!("----");
        enum Atom2 {
            P(ParentAtom2),
            C(ChildAtom2),
        }
        struct ParentAtom2 {
            id: Id4,
            childs: Vec<Atom2>,
        }
        struct ChildAtom2 {
            id: Id4,
            p: Bytes,
        }
        show_size!(Atom2);
        show_size!(ParentAtom2);
        show_size!(ChildAtom2);

        println!("----");
        enum Atom3 {
            P(ParentAtom3),
            C(ChildAtom3),
        }
        struct ParentAtom3 {
            id: Id4,
            childs: VecDeque<Atom3>,
        }
        struct ChildAtom3 {
            id: Id4,
            p: Bytes,
        }
        show_size!(Atom3);
        show_size!(ParentAtom3);
        show_size!(ChildAtom3);
    }
}

//--------------------------------------------------------------------------------
//  パーサー
//
mod _internal {
    use std::sync::atomic::AtomicPtr;

    use bytes::{Buf, Bytes, BytesMut};
    use tracing::{info, trace};

    use super::Atom;
    use crate::{
        error::AtomParseError,
        pcp::{util, ChildAtom, Id4, ParentAtom},
    };

    const HEADER_LENGTH: u32 = 8_u32; // ID4 + SIZE(u32)

    /// Bytes構造体を引数に取りAtomをパースする
    pub(super) fn parse(buf: &mut BytesMut) -> Result<Atom, AtomParseError> {
        // trace!("Parse buf[{}]", buf.len());
        let read_size = parseable(&buf[..])?;
        // trace!("read_size: {}", read_size);

        let mut atom_buf = buf.split_to(read_size as usize);

        let mut atom_buf = atom_buf.freeze();
        let atom = unchecked_parse(&mut atom_buf);
        debug_assert_eq!(atom_buf.len(), 0);
        // trace!("Parse buf[{}]", buf.len());
        Ok(atom)
    }

    pub(super) fn unchecked_parse(buf: &mut Bytes) -> Atom {
        let size_and_parent = (&buf[4..8]).get_u32_le();
        let (is_parent, length) = util::split_packet_size_u32(size_and_parent);

        match is_parent {
            false => {
                // Child
                let payload_length = HEADER_LENGTH + length;
                let child_atom = ChildAtom::new_from_bytes(buf.split_to(payload_length as usize));
                Atom::Child(child_atom)
            }
            true => {
                // Parent
                let id = Id4::from(buf.get_u32());
                let _size_and_parent = buf.get_u32_le(); // already loaded value

                let mut childs: Vec<Atom> = vec![];
                for i in 0..length {
                    let c = unchecked_parse(buf);
                    childs.push(c);
                }

                let parent_atom = ParentAtom::new(id, childs);
                Atom::Parent(parent_atom)
            }
        }
    }

    /// スライスを引数に取りAtomとしてパース可能可能かチェックする
    /// Okの場合帰って来るのは読み込みバイト数
    pub(super) fn parseable(mut buf: &[u8]) -> Result<u32, AtomParseError> {
        let buf_len = buf.len();
        if buf_len < HEADER_LENGTH as usize {
            let x = AtomParseError::NotEnoughRecievedBuffer(HEADER_LENGTH as usize - buf_len);
            return Err(x);
        }

        let id = Id4::from(buf.get_u32());
        let size_and_parent = buf.get_u32_le();
        let (is_parent, length) = util::split_packet_size_u32(size_and_parent);

        let rest_len = buf.len();
        match is_parent {
            false => {
                // ChildAtom
                // trace!("rest_len {}, atom-length: {}", rest_len, length);
                if rest_len < length as usize {
                    Err(AtomParseError::NotEnoughRecievedBuffer(
                        (length as usize) - rest_len,
                    ))
                } else {
                    // TODO: オーバーフローの可能性は無し(lengthに入るのはi32相当なので)
                    Ok(HEADER_LENGTH + length)
                }
            }

            true => {
                // ParentAtom
                let mut start_pos = 0;
                for i in 0..length {
                    let readed = parseable(&buf[start_pos..rest_len])?;
                    start_pos = start_pos + readed as usize;
                }
                // オーバーフローする可能性ある(加算なので・・・)
                Ok(HEADER_LENGTH + start_pos as u32)
            }
        }
    }

    #[cfg(test)]
    mod t {
        use bytes::{Buf, Bytes, BytesMut};

        use crate::{
            error::AtomParseError,
            pcp::{
                atom::_internal::{parse, parseable},
                Atom, ChildAtom, GnuId, Id4, ParentAtom,
            },
        };

        #[test]
        fn test_parseable_childatom() {
            let a = ChildAtom::from((Id4::PCP_ATOM, 1_u16));
            let mut b = BytesMut::new();
            a.write_bytes(&mut b);
            // println!("b: {:?}", &b[..]);
            assert_eq!(b.len(), 4 + 4 + 2);

            // 等価
            assert_eq!(parseable(&b[..]).unwrap(), 10);
            assert_eq!(parseable(&b[0..(4 + 4 + 2)]).unwrap(), 10);

            // 渡す値を減らしてみる
            parseable(&b[0..(0)]).err().map(|e| match e {
                AtomParseError::NotEnoughRecievedBuffer(s) => assert_eq!(s, 8),
                _ => assert!(false),
            });
            // assert_eq!(parseable(&b[0..(0)]).err(), x);
            // Err(AtomParseError::NotEnoughRecievedBuffer(8));

            // assert_eq!(
            //     parseable(&b[0..(4 + 4 + 1)]),
            //     Err(AtomParseError::NotEnoughRecievedBuffer(1))
            // );

            let mut b2 = b.clone();
            assert_eq!(
                parse(&mut b2).unwrap(),
                Atom::Child((Id4::PCP_ATOM, 1_u16).into())
            );

            let id = GnuId::new();
            let a = ChildAtom::from((Id4::PCP_ATOM, id.clone()));
            let mut b = BytesMut::new();
            a.write_bytes(&mut b);

            assert_eq!(parseable(&b[..]).unwrap(), 8 + 16);
            parseable(&b[0..0]).err().map(|e| match e {
                AtomParseError::NotEnoughRecievedBuffer(s) => assert_eq!(s, 8),
                _ => assert!(false),
            });
            parseable(&b[0..8]).err().map(|e| match e {
                AtomParseError::NotEnoughRecievedBuffer(s) => assert_eq!(s, 16),
                _ => assert!(false),
            });
            parseable(&b[0..12]).err().map(|e| match e {
                AtomParseError::NotEnoughRecievedBuffer(s) => assert_eq!(s, 12),
                _ => assert!(false),
            });
        }

        #[test]
        fn test_parseable_parentatom() {
            let a1 = ChildAtom::from((Id4::PCP_ATOM, 1_u16)).into();
            let a2 = ChildAtom::from((Id4::PCP_ATOM, 257_u16)).into();
            let a = ParentAtom::from((Id4::PCP_HELO, vec![a1, a2]));
            let mut b = BytesMut::new();
            a.write_bytes(&mut b);
            assert_eq!(b.len(), 4 + 4 + 10 + 10);

            /*
            //
            assert_eq!(parseable(&b[..]), Ok(28));

            // 親のヘッダのみにすると8バイト欲しいというはず
            assert_eq!(
                parseable(&b[0..(8)]),
                Err(AtomParseError::NotEnoughRecievedBuffer(8))
            );
            // 子1個目までの長さにすると2個目ヘッダの8バイトが欲しいというはず
            assert_eq!(
                parseable(&b[0..(8 + 10)]),
                Err(AtomParseError::NotEnoughRecievedBuffer(8))
            );
            // 子2個めのヘッダまでの長さにすると2個目ペイロードの2バイトが欲しいというはず
            assert_eq!(
                parseable(&b[0..(8 + 10 + 8)]),
                Err(AtomParseError::NotEnoughRecievedBuffer(2))
            ); */
        }

        #[test]
        fn test_parse() {
            let a = ChildAtom::from((Id4::PCP_ATOM, 1_u16));
            let mut bm = BytesMut::new();
            a.write_bytes(&mut bm);

            let a = parse(&mut bm.clone()).unwrap();
            match a {
                Atom::Parent(_) => assert!(false),
                Atom::Child(c) => assert_eq!(c, (Id4::PCP_ATOM, 1_u16).into()),
            };

            // parent
            let a1 = ChildAtom::from((Id4::PCP_ATOM, 1_u16)).into();
            let a2 = ChildAtom::from((Id4::PCP_BCST, 257_u16)).into();
            let a = ParentAtom::from((Id4::PCP_HELO, vec![a1, a2]));
            let mut bm = BytesMut::new();
            a.write_bytes(&mut bm);
            // let mut b = bm.freeze();

            let a = parse(&mut bm.clone()).unwrap();
            match a {
                Atom::Child(_) => assert!(false),
                Atom::Parent(mut p) => {
                    assert_eq!(p.id(), Id4::PCP_HELO);

                    // popなので逆順なのに注意
                    let a = p.childs.pop().unwrap();
                    match a {
                        Atom::Parent(_) => assert!(false),
                        Atom::Child(c) => assert_eq!(c, (Id4::PCP_BCST, 257_u16).into()),
                    };

                    let a = p.childs.pop().unwrap();
                    match a {
                        Atom::Parent(_) => assert!(false),
                        Atom::Child(c) => assert_eq!(c, (Id4::PCP_ATOM, 1_u16).into()),
                    };

                    assert_eq!(p.len(), 0)
                }
            };
        }

        #[ignore = "this is specific check for me"]
        #[test]
        fn test() {
            let b = Bytes::from_static(b"abcdefg");
            let b_len = b.len();

            let s1 = &b[..];
            let mut s2 = &b[..];
            s2.advance(2);
            assert_eq!(s1.len(), b_len);
            assert_eq!(s2.len(), b_len - 2);

            let b = Bytes::from_static(b"abcd");
            let mut s1 = &b[..];
            let x = s1.get_u32();
            println!("{:?}", b);
            println!("{:?}", s1);
            println!("{:?}", x.to_be_bytes());
        }
    }
}

//--------------------------------------------------------------------------------
//  Reader
//
pub async fn read_atom<T>(stream: &mut T, buf: &mut BytesMut) -> Result<Atom, io::Error>
where
    T: AsyncRead + Unpin,
{
    //
    loop {
        match Atom::parseable(&buf[..]) {
            Ok(len) => {
                let mut atom_buf = buf.split_to(len as usize).freeze();
                let atom = Atom::unchecked_parse(&mut atom_buf);
                return Ok(atom);
            }
            Err(AtomParseError::NotEnoughRecievedBuffer(_)) => {
                // 読み込み途中
            }
            Err(
                AtomParseError::NotFoundValue
                | AtomParseError::IdError
                | AtomParseError::Unknown
                | AtomParseError::ValueError,
            ) => return Err(io::Error::from(io::ErrorKind::InvalidData)),
        }

        let result: Result<usize, std::io::Error> = stream.read_buf(buf).await;
        match result {
            Ok(0) => {
                info!("may be EOF");
                return Err(io::Error::from(io::ErrorKind::UnexpectedEof));
            }
            Ok(_size) => {
                trace!("recieved: {:?} bytes", buf.len());
                continue;
            }
            Err(e) => {
                return Err(e);
            }
        }
    }
}
