use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddrV4, Ipv6Addr, SocketAddrV6};
use std::str::FromStr;
use std::sync::Arc;
use network_interface::{NetworkInterface, NetworkInterfaceConfig};
use tokio::net::UdpSocket;
use tokio::time::{sleep, Duration};

const MULTICAST_GROUP: Ipv4Addr = Ipv4Addr::new(239, 15, 16, 17);
const MULTICAST_GROUP_V6: Ipv6Addr = Ipv6Addr::new(0xff00, 0, 0, 0, 0, 0, 0, 0);
const MULTICAST_PORT: u16 = 55100;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    let mut iface_map: HashMap<String, UdpSocket> = HashMap::new();

    let network_interfaces = NetworkInterface::show().unwrap();

    for itf in network_interfaces.iter() {
        println!("{}: {:?}", itf.name, itf);
        for v in itf.addr.iter() {
            if !v.ip().is_loopback() {
                let socket = UdpSocket::bind(format!("{}:{}", v.ip(), MULTICAST_PORT)).await.expect("Could not bind socket");
                println!("Listening on {}", socket.local_addr()?);
                socket.set_broadcast(true)?;
                if v.ip().is_ipv4() {
                    socket.set_multicast_loop_v4(true)?;
                    socket.set_multicast_ttl_v4(1)?;
                    socket.join_multicast_v4(MULTICAST_GROUP, Ipv4Addr::from_str(v.ip().to_string().as_str()).unwrap())?;
                } else {
                    socket.set_multicast_loop_v6(true)?;
                    socket.join_multicast_v6(&MULTICAST_GROUP_V6, 0)?;
                }

                iface_map.insert(v.ip().to_string(), socket);
            }
        }
    }

    let s1 = Arc::new(iface_map);
    let s2 = s1.clone();

    tokio::spawn(async move {
        loop {
            let mut buf = [0u8; 1024];
            for (_, value) in s1.iter() {
                match value.recv_from(&mut buf).await {
                    Ok((size, src)) => {
                        let data = String::from_utf8_lossy(&buf[0..size]);
                        if s1.contains_key(&src.ip().to_string()) {
                            continue;
                        }
                        println!("Received '{}' from {}", data, src);
                    },
                    Err(e) => {
                        eprintln!("Error receiving message: {:?}", e);
                    }
                }
            }
        }
    });

    loop {
        let data = format!("DIS:");
        let addr_v4 = SocketAddrV4::new(MULTICAST_GROUP, MULTICAST_PORT);
        let addr_v6 = SocketAddrV6::new(MULTICAST_GROUP_V6, MULTICAST_PORT, 0, 0);
        for (key, value) in s2.iter() {
            println!("sending from: {}", key);
            if value.local_addr().unwrap().is_ipv4() {
                let _ = value.send_to(data.as_bytes(), addr_v4).await;
            } else {
                let _ = value.send_to(data.as_bytes(), addr_v6).await;
            }
        }
        sleep(Duration::from_secs(1)).await;
    }
}