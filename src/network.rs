use pnet::ipnetwork::IpNetwork;
use pnet::util::MacAddr;
use pnet::datalink;
use pnet::packet::arp::{ArpHardwareTypes, ArpOperations, MutableArpPacket, ArpPacket, };
use pnet::packet::ethernet::{EtherTypes, MutableEthernetPacket, EthernetPacket};
use pnet::datalink::Channel::Ethernet;
use pnet::packet::MutablePacket;
use pnet::packet::Packet;

use std::time::Duration;
use std::net::Ipv4Addr;
use dict::{Dict, DictIface};
use std::thread;

#[derive(Debug, Clone)]
pub struct InterfaceInfo {
    pub name: String,
    pub mac: MacAddr,
    pub ip: Ipv4Addr
}

pub fn get_available_interfaces() -> Dict<InterfaceInfo> {
    let interfaces = datalink::interfaces();
    let mut interface_dict = Dict::<InterfaceInfo>::new();

    println!("Interfaces:");

    for interface in interfaces {
        println!("Name: {}\nIndex: {}", interface.name, interface.index);
        
        if let Some(mac) = interface.mac {
            println!("MAC: {:?}", mac);
            
            if !interface.ips.is_empty() {
                if let IpNetwork::V4(address) = interface.ips[0] {
                    let info = InterfaceInfo {
                        name: interface.name.clone(), 
                        mac, 
                        ip: address.ip()
                    };

                    interface_dict.add(interface.index.to_string(), info);
                    println!("Address: {:?}", address.ip());
                }
            } else {
                println!("Address: No IP assigned");
            }
        } else {
            println!("MAC: Failed to get");
        }
        println!("------------------------");
    }

    interface_dict
}

fn build_arp_packet(
    source_mac: MacAddr,
    source_ip: Ipv4Addr,
    target_ip: Ipv4Addr) -> Vec<u8> {
    
    // calc buffer size
    let size = 28 + 14;

    let mut buffer = vec!(0u8; size);

    let mut eth_packet = MutableEthernetPacket::new(&mut buffer).unwrap();

    eth_packet.set_destination(MacAddr::broadcast());
    eth_packet.set_source(source_mac);
    eth_packet.set_ethertype(EtherTypes::Arp);

    let mut arp_packet = MutableArpPacket::new(eth_packet.payload_mut()).unwrap();

    arp_packet.set_hardware_type(ArpHardwareTypes::Ethernet);
    arp_packet.set_protocol_type(EtherTypes::Ipv4);
    arp_packet.set_hw_addr_len(6);
    arp_packet.set_proto_addr_len(4);
    arp_packet.set_operation(ArpOperations::Request);

    arp_packet.set_sender_hw_addr(source_mac);
    arp_packet.set_sender_proto_addr(source_ip);

    arp_packet.set_target_hw_addr(MacAddr::zero());
    arp_packet.set_target_proto_addr(target_ip);

    buffer
}

fn discover_listener_func(interface_struct: InterfaceInfo, subnet: String, mut rx: Box<dyn pnet::datalink::DataLinkReceiver>) {
    loop {
        let recv = rx.next();

        match recv {
            Ok(packet_bytes) => {
                if let Some(eth_packet) = EthernetPacket::new(packet_bytes) {
                    if eth_packet.get_ethertype() == EtherTypes::Arp {
                        if let Some(arp_packet) = ArpPacket::new(eth_packet.payload()) {
                            if arp_packet.get_operation() == ArpOperations::Reply {
                                println!("[+] Hosts MAC: {}", arp_packet.get_sender_hw_addr());
                                println!("[+] Hosts IP: {}", arp_packet.get_sender_proto_addr());
                            }
                        }
                    }
                }
            },
            Err(e) => {
                if e.kind() == std::io::ErrorKind::TimedOut || e.kind() == std::io::ErrorKind::WouldBlock {
                    println!("[*] No more ARP packets received");
                    break;
                }
            }
        }
    }
}

pub fn scan_for_hosts(interface_struct: &InterfaceInfo, subnet: String) {
    let interfaces = datalink::interfaces();

    let interface = interfaces.into_iter().find(|ifname| ifname.name == interface_struct.name).unwrap();

    let mut config = pnet::datalink::Config::default();

    config.read_timeout = Some(Duration::from_secs(5));

    // opening channel to hardware card
    let (mut tx, mut rx) = match datalink::channel(&interface, config) {
        Ok(Ethernet(tx, rx)) => (tx, rx),
        Ok(_) => panic!("[-] Failed to create a pipe"),
        Err(e) => panic!("[-] Failed to open channel: {}", e)
    };

    let interface_struct_thread = interface_struct.clone();
    let subnet_thread = subnet.clone();

    let listener_handle = thread::spawn(move || {discover_listener_func(interface_struct_thread, subnet_thread, rx)});

    // sending arp packets
    for sub in 1..255 {
        let target_ip: Ipv4Addr = format!("{}.{}", subnet, sub).parse().expect("[-] Failed to parse");

        let arp_packet = build_arp_packet(interface_struct.mac, interface_struct.ip, target_ip);

        match tx.send_to(&arp_packet, None) {
            Some(Ok(())) => {},
            Some(Err(e)) => eprintln!("[-] Failed to send ARP packet"),
            None => eprintln!("[-] Channel in invalid state")
        }

        thread::sleep(Duration::from_millis(3));
    }

    listener_handle.join().unwrap();
    
    println!("Scan completed");
}