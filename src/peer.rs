use crate::parsing::Torrent;
use crate::types::{PeerMessage, PeerMessageType};
use anyhow::Result;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

const KB_16: u32 = 16_384;

#[derive(Clone)]
pub struct Tracker {
    pub torrent: Torrent,
    pub peer_list: Vec<String>,
}

impl Tracker {
    pub async fn new(torrent: Torrent) -> Result<Tracker> {
        let peer_list = torrent.get_peer_list().await?;
        Ok(Tracker { torrent, peer_list })
    }

    pub async fn create_handshake(&self, peer: Option<&String>) -> Result<(TcpStream, [u8; 68])> {
        let peer = match peer {
            Some(s) => s,
            None => &self.peer_list[0],
        };

        let hash_info = self.torrent.get_info_hash()?;

        let mut handshake: Vec<u8> = Vec::new();
        handshake.push(19);
        handshake.extend(b"BitTorrent protocol");
        handshake.extend([0; 8]);
        handshake.extend(hash_info);
        handshake.extend([0; 20]);

        let mut stream = TcpStream::connect(peer).await?;
        stream.write(&handshake).await?;
        let mut buffer = [0; 68];
        stream.read_exact(&mut buffer).await?;

        Ok((stream, buffer))
    }

    pub async fn download_piece(
        &mut self,
        socket: &mut TcpStream,
        piece_index: usize,
    ) -> Result<Vec<u8>> {
        // Await for a bitfield message
        let msg = PeerMessage::from_socket(socket).await?;
        if msg.id != PeerMessageType::Bitfield {
            panic!("No Bitfield Message Received");
        }

        // Send an interested message
        socket
            .write_all(
                &PeerMessage {
                    id: PeerMessageType::Interested,
                    length: 1,
                    payload: vec![],
                }
                .to_bytes(),
            )
            .await?;

        // Await for a unchoke message
        let msg = PeerMessage::from_socket(socket).await?;
        if msg.id != PeerMessageType::Unchoke {
            panic!("No Unchoke Message Received");
        }

        // Setting Variable to read piece
        let mut offset = 0;
        let file_length = self.torrent.info.length;
        let piece_length = self.torrent.info.piece_length;
        let length = (file_length - (piece_length * piece_index)).min(piece_length) as u32;
        let mut chuck: Vec<u8> = Vec::new();

        // Run loop to piece together piece
        while offset < length {
            // Set payload with piece index, offset and block size
            let block_size: u32 = KB_16.min(length - offset);
            let mut payload: Vec<u8> = Vec::new();
            payload.extend((piece_index as u32).to_be_bytes());
            payload.extend(offset.to_be_bytes());
            payload.extend(block_size.to_be_bytes());

            // Construct full message
            eprintln!("Block Length: {block_size}, Piece Offset: {offset}, Total: {length}");
            let req = PeerMessage {
                id: PeerMessageType::Request,
                // INFO: The + 1 comes from the piece_index which an extra byte
                length: (payload.len() + 1) as u32,
                payload,
            };

            // Write payload
            eprintln!("Waiting for piece...");
            socket.write_all(&req.to_bytes()).await?;

            // Receive piece message
            let msg = PeerMessage::from_socket(socket).await?;
            if PeerMessageType::Piece != msg.id {
                panic!("No Piece Message Received");
            }

            // Extend chuck
            chuck.extend(&msg.payload[8..]);
            offset += block_size;
        }

        Ok(chuck)
    }

    pub async fn download(&self) -> Result<Vec<u8>> {
        let number_of_pieces = self.torrent.info.pieces.len() / 20;
        let mut buffer = vec![];
        let mut handles = vec![];

        for piece_index in 0..number_of_pieces {
            let mut t = self.clone();
            let handle = tokio::spawn(async move {
                let mut i = piece_index;
                loop {
                    let peer = Some(&t.peer_list[i % t.peer_list.len()]);
                    let response = t.create_handshake(peer).await;
                    if response.is_err() {
                        eprintln!("HandShake Failure: {:?}", response.unwrap_err());
                        i += 1;
                        continue;
                    }
                    let (mut socket, _) = response.unwrap();
                    let response = t.download_piece(&mut socket, piece_index).await;
                    if response.is_err() {
                        eprintln!("Piece Failure: {:?}", response.unwrap_err());
                        i += 1;
                        continue;
                    }
                    let piece = response.unwrap();
                    return piece;
                }
            });
            handles.push(handle);
        }

        for h in handles {
            let v = h.await?;
            buffer.extend(&v[8..]);
        }

        Ok(buffer)
    }
}
