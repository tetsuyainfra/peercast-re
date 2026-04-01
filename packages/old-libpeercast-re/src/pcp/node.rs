use std::net::SocketAddr;

#[derive(Debug, Clone)]
pub struct Node {
    pub addr: SocketAddr,
    pub global_addr: SocketAddr,
}

impl Node {
    pub fn addr(mut self, a: SocketAddr) -> Self {
        self.addr = a;
        self
    }
}

#[cfg(test)]
mod t {
    use super::*;

    #[test]
    fn test() {
        let node = Node {
            addr: "127.0.0.1:16".parse().unwrap(),
            global_addr: "127.0.0.1:16".parse().unwrap(),
        };

        let node = node.addr("127.0.0.2:1".parse().unwrap());
    }
}
