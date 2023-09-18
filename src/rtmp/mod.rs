use tokio::sync::mpsc;

pub mod connection;
pub mod rtmp_connection;
pub mod stream_manager;

#[derive(PartialEq)]
pub enum ConnectionAction {
    None,
    Disconnect,
}

#[derive(PartialEq, Debug, Clone)]
pub enum State {
    Waiting,
    Connected {
        app_name: String,
    },
    PublishRequested {
        app_name: String,
        stream_key: String,
        request_id: u32,
    },
    Publishing {
        app_name: String,
        stream_key: String,
    },
    PlaybackRequested {
        app_name: String,
        stream_key: String,
        request_id: u32,
        stream_id: u32,
    },
    Playing {
        app_name: String,
        stream_key: String,
        stream_id: u32,
    },
}

/// Sends a message over an unbounded receiver and returns true if the message was sent
/// or false if the channel has been closed.
fn send<T>(sender: &mpsc::UnboundedSender<T>, message: T) -> bool {
    match sender.send(message) {
        Ok(_) => true,
        Err(_) => false,
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[ignore = "this is main"]
    #[crate::test]
    async fn main() -> Result<(), std::io::Error> {
        let manager_sender = stream_manager::start();

        println!("Listening for connections on port 11935");
        let listener = tokio::net::TcpListener::bind("0.0.0.0:11935").await?;
        let mut current_id = 0;

        loop {
            let (stream, connection_info) = listener.accept().await?;

            let connection = connection::Connection::new(current_id, manager_sender.clone());
            println!(
                "Connection {}: Connection received from {}",
                current_id,
                connection_info.ip()
            );

            tokio::spawn(connection.start_handshake(stream));
            current_id = current_id + 1;
        }
    }
}
