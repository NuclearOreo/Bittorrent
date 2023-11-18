use anyhow::{Context, Result};
use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;

#[derive(Debug)]
pub struct PeerMessage {
    pub length: u32,
    pub id: PeerMessageType,
    pub payload: Vec<u8>,
}

impl PeerMessage {
    pub async fn from_socket(stream: &mut TcpStream) -> Result<Self> {
        let mut buffer = [0; 4];
        stream.read_exact(&mut buffer).await?;
        let msg_len = u32::from_be_bytes(buffer);
        let mut buffer = vec![0; msg_len as usize];
        stream.read_exact(&mut buffer).await?;
        let (id, payload) = buffer.split_first().context("Failed split.")?;
        Ok(Self {
            length: msg_len,
            id: id.into(),
            payload: Vec::from(payload),
        })
    }
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend(&self.length.to_be_bytes());
        bytes.push(self.id.clone().into());
        bytes.extend(&self.payload);
        bytes
    }
}

#[derive(Eq, PartialEq, Clone, Debug)]
pub enum PeerMessageType {
    Bitfield,
    Interested,
    Unchoke,
    Request,
    Piece,
}
impl From<&u8> for PeerMessageType {
    fn from(value: &u8) -> Self {
        match value {
            5 => PeerMessageType::Bitfield,
            2 => PeerMessageType::Interested,
            1 => PeerMessageType::Unchoke,
            6 => PeerMessageType::Request,
            7 => PeerMessageType::Piece,
            _ => panic!("Don't know about this PeerMessageType"),
        }
    }
}
impl From<PeerMessageType> for u8 {
    fn from(value: PeerMessageType) -> Self {
        match value {
            PeerMessageType::Bitfield => 5,
            PeerMessageType::Interested => 2,
            PeerMessageType::Unchoke => 1,
            PeerMessageType::Request => 6,
            PeerMessageType::Piece => 7,
        }
    }
}
