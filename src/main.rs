mod parsing;
mod peer;
mod types;

use anyhow::Result;
use clap::{Parser, Subcommand};
use parsing::{Torrent, ENCODED};
use peer::Tracker;
use tokio::{self, io::AsyncWriteExt};

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
#[clap(rename_all = "snake_case")]
enum Commands {
    Decode {
        encoded_value: String,
    },
    Info {
        path: String,
    },
    Peers {
        path: String,
    },
    Handshake {
        path: String,
        peer_url: String,
    },
    DownloadPiece {
        #[arg(short, long, value_name = "FILE-PATH")]
        out: String,
        path: String,
        piece_index: usize,
    },
    Download {
        #[arg(short, long, value_name = "FILE-PATH")]
        out: String,
        path: String,
    },
}

// Usage: your_bittorrent.sh decode "<encoded_value>"
#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Decode { encoded_value } => {
            let encoded_value = ENCODED::String(encoded_value);
            let decoded_value = Torrent::decode_bencoded_value(encoded_value)?;
            println!("{}", decoded_value.to_string());
        }
        Commands::Info { path } => {
            let torrent = Torrent::get_torrent(&path)?;
            let hash = torrent.get_info_hash()?;
            let hash_string = hex::encode(hash);

            println!(
                "Tracker URL: {}\nLength: {}\nInfo Hash: {}\nPiece Length: {}\nPiece Hashes:",
                torrent.announce, torrent.info.length, hash_string, torrent.info.piece_length
            );
            for i in (0..torrent.info.pieces.len()).step_by(20) {
                println!("{}", hex::encode(&torrent.info.pieces[i..i + 20]));
            }
        }
        Commands::Peers { path } => {
            let torrent = Torrent::get_torrent(&path)?;
            let peer_list = torrent.get_peer_list().await?;

            for p in peer_list.iter() {
                println!("{}", p);
            }
        }
        Commands::Handshake { path, peer_url } => {
            let torrent = Torrent::get_torrent(&path)?;
            let tracker = Tracker::new(torrent.clone()).await?;
            let (_, buffer) = tracker.create_handshake(Some(&peer_url)).await?;
            let peer_id = hex::encode(&buffer[buffer.len() - 20..]);
            println!("Peer ID: {}", peer_id);
        }
        Commands::DownloadPiece {
            out,
            path,
            piece_index,
        } => {
            let torrent = Torrent::get_torrent(&path)?;
            let mut tracker: Tracker = Tracker::new(torrent).await?;
            let (mut socket, _) = tracker.create_handshake(None).await?;
            let piece = tracker.download_piece(&mut socket, piece_index).await?;
            socket.shutdown().await?;
            std::fs::write(out.clone(), piece)?;
            println!("Piece {} downloaded to {}", piece_index, out);
        }
        Commands::Download { out, path } => {
            let torrent = Torrent::get_torrent(&path)?;
            let tracker: Tracker = Tracker::new(torrent).await?;
            let buffer = tracker.download().await?;
            std::fs::write(out.clone(), buffer)?;
            println!("Downloaded {} to {}.", path, out);
        }
    }

    Ok(())
}
