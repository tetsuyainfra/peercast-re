fn main() {}

#[cfg(test)]
mod test {
    use bytes::BufMut;
    use tokio_util::bytes::BytesMut;

    #[test]
    fn test_bytes_clone() {
        let mut b1 = BytesMut::with_capacity(4);

        b1.put(&b"a"[..]);
        b1.put(&b"bcd"[..]);
        assert_eq!(b1.capacity(), 4);

        // 確保された容量を超えると自動的に拡張(x2)される
        b1.put(&b"x"[..]);
        assert_ne!(b1.capacity(), 4);
        assert_eq!(b1.capacity(), 8);
    }
}
