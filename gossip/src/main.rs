use std::{
    collections::{HashMap, HashSet},
    net::{SocketAddr, ToSocketAddrs},
    process::exit,
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};
mod utils;
use solana_client::connection_cache::{self, ConnectionCache};
use solana_gossip::{
    cluster_info::{ClusterInfo, Node},
    contact_info::ContactInfo,
    gossip_service::GossipService,
};
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer, timing::timestamp};
use solana_streamer::socket::SocketAddrSpace;
use tracing::{info, Level};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{fmt::format::FmtSpan, prelude::*, EnvFilter};

fn setup_logging() -> Result<(), Box<dyn std::error::Error>> {
    // Create logs directory
    std::fs::create_dir_all("logs")?;

    // Set up file appender with daily rotation
    let file_appender = RollingFileAppender::new(Rotation::DAILY, "logs", "solana-gossip.log");

    // Create an environment filter that includes Solana's internal traces
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,solana_gossip=trace,solana_cluster_info=trace"));

    // Set up the subscriber with both console and file output
    tracing_subscriber::registry()
        .with(env_filter)
        .with(
            tracing_subscriber::fmt::layer()
                .with_file(true)
                .with_line_number(true)
                .with_thread_ids(true)
                .with_target(true)
                .with_writer(file_appender)
                .with_ansi(false),
        )
        .with(tracing_subscriber::fmt::layer().compact())
        .init();

    Ok(())
}

fn main() {
    // Initialize logging first
    setup_logging().unwrap_or_else(|err| {
        eprintln!("Failed to set up logging: {}", err);
        exit(1);
    });

    info!("Starting Solana gossip service...");

    let id_pair = Arc::new(Keypair::new());
    let shred_version: u16 = 27799;
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
        info!(
            "Contacting {} to determine validator's public IP address",
            entrypoint_addr
        );

        solana_net_utils::get_public_ip_addr(entrypoint_addr).map_or_else(
            |err| {
                eprintln!("Failed to contact cluster entrypoint {entrypoint_addr}: {err}");
                None
            },
            Some,
        )
    });

    let gossip_host = gossip_host.unwrap_or_else(|| {
        eprintln!("Unable to determine the validator's public IP address");
        exit(1)
    });

    let mut node = Node::new_localhost_with_pubkey(&id_pair.try_pubkey().unwrap());
    node.info.set_shred_version(shred_version);
    node.info.set_wallclock(timestamp());
    let cluster_info = ClusterInfo::new(node.info, id_pair, SocketAddrSpace::Unspecified);

    let cluster_entrypoints = entrypoint_addrs
        .iter()
        .map(ContactInfo::new_gossip_entry_point)
        .collect::<Vec<_>>();

    let cluster_info = Arc::new(cluster_info);
    cluster_info.set_entrypoints(cluster_entrypoints);
    info!("Cluster info peers: {:?}", cluster_info.all_peers());

    let connection_cache = ConnectionCache::new_quic("connection_cache_banking_bench_quic", 1);
    let exit = Arc::new(AtomicBool::new(false));

    let gossip_validators: HashSet<Pubkey> = HashSet::new();
    info!("Starting gossip service...");
    let gossip_service = GossipService::new(
        &cluster_info,
        None,
        node.sockets.gossip,
        Some(gossip_validators.clone()),
        true,
        None,
        exit.clone(),
    );

    gossip_service.join();
}
