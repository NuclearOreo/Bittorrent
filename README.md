# Torrent Exploration

A CLI project exploring how to download a file be parsing a torrent file and running the protocol.

## CLI Commands Examples

```sh
# Decode Bencode
cargo run -- decode 5:hello 

# Decode Integer Bencode 
cargo run -- decode i52e

# Decode List Bencode 
cargo run -- decode l5:helloi52ee

# Decode Dictionary Bencode 
cargo run -- decode d3:foo3:bar5:helloi52ee

# Parse Torrent file
cargo run -- info sample.torrent

# List Peer from Torrent file
cargo run -- peers sample.torrent

# Handshake with Peer 
cargo run -- handshake sample.torrent <peer_ip>:<peer_port>

# Download a Piece
cargo run -- download_piece -o /tmp/test-piece-0 sample.torrent 0

# Download whole file
cargo run -- download -o /tmp/test.txt sample.torrent
```