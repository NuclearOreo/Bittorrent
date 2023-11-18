use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_bencode::value::Value as benEnum;
use serde_json::Value as JsonEnum;
use sha1::{Digest, Sha1};
use std::fs::File;
use std::io::prelude::*;

pub enum ENCODED {
    Bytes(Vec<u8>),
    String(String),
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Clone)]
pub struct Torrent {
    pub announce: String,
    pub info: Info,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Clone)]
pub struct Info {
    pub name: String,
    #[serde(rename = "piece length")]
    pub piece_length: usize,
    #[serde(with = "serde_bytes")]
    pub pieces: Vec<u8>,
    pub length: usize,
}

impl Torrent {
    pub fn new(map: &JsonEnum) -> Result<Self> {
        let announce = match map["announce"].as_str() {
            Some(s) => s.to_string(),
            None => panic!("Unable to a get announce"),
        };

        let name = match map["info"]["name"].as_str() {
            Some(s) => s.to_string(),
            None => panic!("Unable to a get name"),
        };

        let piece_length = match map["info"]["piece length"].as_u64() {
            Some(s) => s as usize,
            None => panic!("Unable to a get piece length"),
        };

        let length = match map["info"]["length"].as_u64() {
            Some(s) => s as usize,
            None => panic!("Unable to a get piece length"),
        };

        let pieces = match map["info"]["pieces"].as_str() {
            Some(s) => hex::decode(s),
            None => panic!("Unable to a get announce"),
        }?;

        Ok(Torrent {
            announce,
            info: Info {
                name,
                piece_length,
                length,
                pieces,
            },
        })
    }

    pub fn get_torrent(torrent_file: &String) -> Result<Torrent> {
        let path = torrent_file;
        let mut file = File::open(path)?;
        let mut contents = Vec::new();
        file.read_to_end(&mut contents)?;

        let encoded_value = ENCODED::Bytes(contents.clone());
        let decoded_value = Self::decode_bencoded_value(encoded_value)?;

        Ok(Self::new(&decoded_value)?)
    }

    pub fn convert(value: benEnum) -> Result<JsonEnum> {
        match value {
            benEnum::Bytes(b) => {
                let string = match String::from_utf8(b.clone()) {
                    Ok(s) => s,
                    Err(_) => hex::encode(&b),
                };
                return Ok(JsonEnum::String(string));
            }
            benEnum::Int(v) => {
                return Ok(JsonEnum::Number(serde_json::Number::from(v)));
            }
            benEnum::List(l) => {
                let res: Result<Vec<JsonEnum>> = l.into_iter().map(|i| Self::convert(i)).collect();
                return Ok(JsonEnum::Array(res?));
            }
            benEnum::Dict(d) => {
                let mut m = serde_json::Map::new();

                for (k, v) in d.into_iter() {
                    let sk = String::from_utf8(k)?;
                    let vv = Self::convert(v)?;
                    m.insert(sk, vv);
                }

                return Ok(JsonEnum::Object(m));
            }
        }
    }

    pub fn decode_bencoded_value(encoded_value: ENCODED) -> Result<JsonEnum> {
        let value = match encoded_value {
            ENCODED::Bytes(b) => serde_bencode::from_bytes(&b),
            ENCODED::String(s) => serde_bencode::from_str(&s),
        }?;
        Self::convert(value)
    }

    pub fn get_info_hash(&self) -> Result<Vec<u8>> {
        let bencoded_info = serde_bencode::to_bytes(&self.info)?;
        let mut hasher = Sha1::new();
        hasher.update(bencoded_info);
        let hash = hasher.finalize();
        Ok(hash.to_vec())
    }

    pub async fn get_peer_list(&self) -> Result<Vec<String>> {
        let info_hash = self.get_info_hash()?;
        let percent_encoded_hash: String = info_hash
            .iter()
            .map(|byte| format!("%{:02x}", byte))
            .collect();

        let peer_id = "00000000000000000000";
        let tracker = format!(
            "{}?info_hash={}&peer_id={}&port=6881&uploaded=0&downloaded=0&left={}&compact=1",
            self.announce, percent_encoded_hash, peer_id, self.info.length
        );
        let response = reqwest::get(tracker).await?;
        let body = response.bytes().await?;
        let encode_value = ENCODED::Bytes(body.to_vec());

        let map = Torrent::decode_bencoded_value(encode_value)?;

        let peer_list = match map["peers"].as_str() {
            Some(s) => {
                let mut peer_list = vec![];
                let bytes = hex::decode(s)?;
                for i in (0..bytes.len()).step_by(6) {
                    let ip = [
                        bytes[i].to_string(),
                        bytes[i + 1].to_string(),
                        bytes[i + 2].to_string(),
                        bytes[i + 3].to_string(),
                    ]
                    .join(".");
                    let port = ((bytes[i + 4] as u16) << 8) | (bytes[i + 5] as u16);
                    peer_list.push(format!("{}:{}", ip, port));
                }
                peer_list
            }
            None => panic!("Unable to get peers"),
        };

        Ok(peer_list)
    }
}
