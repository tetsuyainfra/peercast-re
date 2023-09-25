use std::net::{IpAddr, Ipv4Addr};

pub struct ServerConfig {
    server_address: Vec<IpAddr>,
    server_port: u16,

    // http_server:    HttpServerConfig,
    rtmp_address: IpAddr,
    rtmp_port: u16,
}

struct Server {
    config: ServerConfig,
}

impl Server {
    fn change_peca_address(&mut self) -> bool {
        true
    }
    fn change_peca_port(&mut self) -> bool {
        true
    }

    fn change_rtmp_address(&mut self) -> bool {
        true
    }
    fn change_rtmp_port(&mut self) -> bool {
        true
    }

    // fn save_config() {}
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            server_address: Default::default(),
            server_port: Default::default(),
            rtmp_address: "127.0.0.1".parse().unwrap(),
            rtmp_port: 11935,
        }
    }
}

// impl From<ServerConfig> for Server {}
#[cfg(test)]
mod t {
    use super::*;

    #[test]
    fn default() {
        let server = ServerConfig::default();
    }
}
