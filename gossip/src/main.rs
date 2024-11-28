use std::{
    net::{SocketAddr, ToSocketAddrs},
    process::exit,
    sync::Arc,
};
mod utils;
use solana_client::connection_cache::{self, ConnectionCache};
use solana_gossip::{
    cluster_info::{ClusterInfo, Node},
    contact_info::ContactInfo,
};
use solana_sdk::{signature::Keypair, signer::Signer};
use solana_streamer::socket::SocketAddrSpace;

fn main() {
    let entrypoint_urls: Vec<&str> = vec![
        "entrypoint.testnet.solana.com:8001",
        "entrypoint2.testnet.solana.com:8001",
        "entrypoint3.testnet.solana.com:8001",
    ];

    let entrypoint_addrs = entrypoint_urls
        .iter()
        .map(|addr| utils::parse_host_port(addr).unwrap())
        .collect::<Vec<SocketAddr>>();

    let mut order: Vec<_> = (0..entrypoint_addrs.len()).collect();

    let gossip_host = order.into_iter().find_map(|i| {
        let entrypoint_addr = &entrypoint_addrs[i];
        // info!(
        //     "Contacting {} to determine the validator's public IP address",
        //     entrypoint_addr
        // );
        solana_net_utils::get_public_ip_addr(entrypoint_addr).map_or_else(
            |err| {
                eprintln!("Failed to contact cluster entrypoint {entrypoint_addr}: {err}");
                None
            },
            Some,
        )
    });

    gossip_host.unwrap_or_else(|| {
        eprintln!("Unable to determine the validator's public IP address");
        exit(1)
    });

    let cluster_info = {
        let keypair = Arc::new(Keypair::new());
        let node = Node::new_localhost_with_pubkey(&keypair.try_pubkey().unwrap());
        ClusterInfo::new(node.info, keypair, SocketAddrSpace::Unspecified)
    };

    let cluster_entrypoints = entrypoint_addrs
        .iter()
        .map(ContactInfo::new_gossip_entry_point)
        .collect::<Vec<_>>();

    let cluster_info = Arc::new(cluster_info);

    println!("cluster_info: {:?}", cluster_info.all_peers());
    let connection_cache = ConnectionCache::new_quic("connection_cache_banking_bench_quic", 1);
}
