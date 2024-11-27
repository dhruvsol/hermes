use std::sync::Arc;

use solana_client::connection_cache::{self, ConnectionCache};
use solana_gossip::cluster_info::{ClusterInfo, Node};
use solana_sdk::{signature::Keypair, signer::Signer};
use solana_streamer::socket::SocketAddrSpace;
fn main() {
    let cluster_info = {
        let keypair = Arc::new(Keypair::new());
        let node = Node::new_localhost_with_pubkey(&keypair.try_pubkey().unwrap());
        ClusterInfo::new(node.info, keypair, SocketAddrSpace::Unspecified)
    };

    let cluster_info = Arc::new(cluster_info);
    println!("cluster_info: {:?}", cluster_info.all_peers());
    let connection_cache = ConnectionCache::new_quic("connection_cache_banking_bench_quic", 1);
}
