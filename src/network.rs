use pnet::ipnetwork::IpNetwork;
use pnet::util::MacAddr;
use pnet::datalink;
use pnet::packet::arp::{ArpHardwareTypes, ArpOperations, MutableArpPacket, ArpPacket, };
use pnet::packet::ethernet::{EtherTypes, MutableEthernetPacket, EthernetPacket};
use pnet::datalink::Channel::Ethernet;
use pnet::packet::MutablePacket;
use pnet::packet::Packet;
use etherparse::PacketBuilder;
use pnet::packet::ipv4::Ipv4Packet;
use pnet::packet::tcp::{TcpPacket, TcpFlags};

use std::time::Duration;
use std::net::Ipv4Addr;
use dict::{Dict, DictIface};
use std::thread;
use std::collections::HashSet;
use std::io::ErrorKind;
use std::net::TcpStream;
use std::io::{Read, Write};

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
    let mut discovered_ips = HashSet::new();

    discovered_ips.insert(interface_struct.ip);

    loop {
        let recv = rx.next();

        match recv {
            Ok(packet_bytes) => {
                if let Some(eth_packet) = EthernetPacket::new(packet_bytes) {
                    if eth_packet.get_ethertype() == EtherTypes::Arp {
                        if let Some(arp_packet) = ArpPacket::new(eth_packet.payload()) {
                            if arp_packet.get_operation() == ArpOperations::Reply {
                                if discovered_ips.insert(arp_packet.get_sender_proto_addr()) {
                                    println!("[+] Hosts MAC: {}", arp_packet.get_sender_hw_addr());
                                    println!("[+] Hosts IP: {}", arp_packet.get_sender_proto_addr());
                                }
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

        // Because if its the host itself itll just send a packet through loopback addr and then it gets stuck in infinite loop
        if target_ip == interface_struct.ip  {
            println!("[+] Hosts MAC: {}", interface_struct.mac);
            println!("[+] Hosts IP: {}", interface_struct.ip);

            continue;
        }
        else {
            let arp_packet = build_arp_packet(interface_struct.mac, interface_struct.ip, target_ip);

            match tx.send_to(&arp_packet, None) {
                Some(Ok(())) => {},
                Some(Err(e)) => eprintln!("[-] Failed to send ARP packet"),
                None => eprintln!("[-] Channel in invalid state")
            }

            thread::sleep(Duration::from_millis(3));
        }
    }

    listener_handle.join().unwrap();
    
    println!("Scan completed");
}

fn get_mac_addr(arp_packet: Vec<u8>, interface_struct: &InterfaceInfo, target: &Ipv4Addr) -> Option<MacAddr>{
    let interfaces = datalink::interfaces();

    let interface = interfaces.into_iter().find(|ifname| ifname.name == interface_struct.name).unwrap();

    let mut config = pnet::datalink::Config::default();

    config.read_timeout = Some(Duration::from_secs(2));

    // opening channel to hardware card
    let (mut tx, mut rx) = match datalink::channel(&interface, config) {
        Ok(Ethernet(tx, rx)) => (tx, rx),
        Ok(_) => panic!("[-] Failed to create a pipe"),
        Err(e) => panic!("[-] Failed to open channel: {}", e)
    };

    match tx.send_to(&arp_packet, None) {
        Some(Ok(())) => {},
        Some(Err(e)) => eprintln!("[-] Failed to send ARP packet"),
        None => eprintln!("[-] Channel in invalid state")
    }

    let mut mac_addr: Option<MacAddr> = None;

    if interface_struct.ip == *target {
        return Some(interface_struct.mac);
    }

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

                                mac_addr = Some(arp_packet.get_sender_hw_addr());

                                break;
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

    return mac_addr
}

fn scan_port(interface_struct: &InterfaceInfo, target: &String, mac_addr: &MacAddr, port: u16, services_scan: bool, os_scan: bool) {
    let target_ipv4: std::net::Ipv4Addr = target.parse().expect("[-] Invalid target IP");

    let builder = PacketBuilder::ethernet2(
        interface_struct.mac.octets(),
        (*mac_addr).octets(),
    )
    .ipv4(
        interface_struct.ip.octets(),
        target_ipv4.octets(),
        67
    )
    .tcp(
        8000, // will also be randomized in the future
        port,
        1, // this is sequence num, should be randomized in future
        26180
    )
    .syn();

    let payload = [];

    let mut packet_bytes = Vec::new();

    builder.write(&mut packet_bytes, &payload).unwrap();

    let interfaces = datalink::interfaces();

    let interface = interfaces.into_iter().find(|ifname| ifname.name == interface_struct.name).unwrap();

    let mut config = pnet::datalink::Config::default();

    config.read_timeout = Some(Duration::from_secs(2));

    let (mut tx, mut rx) = match datalink::channel(&interface, config) {
        Ok(Ethernet(tx, rx)) => (tx, rx),
        Ok(_) => panic!("[-] Failed to create a pipe"),
        Err(e) => panic!("[-] Failed to open channel: {}", e)
    };

    match tx.send_to(&packet_bytes, None) {
        Some(Ok(())) => {},
        Some(Err(e)) => eprintln!("[-] Failed to send ARP packet"),
        None => eprintln!("[-] Channel in invalid state")
    };

    loop {
        let recv = rx.next();

        match recv {
            Ok(packet_bytes) => {
                if let Some(eth_packet) = EthernetPacket::new(packet_bytes) {
                    if eth_packet.get_ethertype() != EtherTypes::Ipv4  {continue;}

                    if let Some(ipv4_packet) = Ipv4Packet::new(eth_packet.payload()) {
                        if ipv4_packet.get_source() != target_ipv4 {continue;}

                        if ipv4_packet.get_next_level_protocol() == pnet::packet::ip::IpNextHeaderProtocols::Icmp {
                            if let Some(icmp_packet) = pnet::packet::icmp::IcmpPacket::new(ipv4_packet.payload()) {
                                if icmp_packet.get_icmp_type() == pnet::packet::icmp::IcmpTypes::DestinationUnreachable {
                                    println!("Port {} is FILTERED (Received ICMP Destination Unreachable)", port);
                                    break;
                                }
                            }
                        }

                        if ipv4_packet.get_next_level_protocol() == pnet::packet::ip::IpNextHeaderProtocols::Tcp {
                            if let Some(tcp_packet) = TcpPacket::new(ipv4_packet.payload()) {
                                if tcp_packet.get_destination() != 8000 {continue;}

                                //println!("{:?}", tcp_packet);
                                
                                let flags = tcp_packet.get_flags();

                                if (flags & TcpFlags::SYN != 0) && (flags & TcpFlags::ACK != 0) {
                                    println!("Port {} is OPEN", port);
                                }
                                else if (flags & TcpFlags::RST != 0) {
                                    println!("Port {} is CLOSED", port);
                                }

                                if os_scan == true {
                                    let ttl = ipv4_packet.get_ttl();

                                    if ttl < ((64 as f32) * 0.8) as u8 {
                                        println!("OS of target calculated from PORT {} is: Microcontroller/Embedded Device", port);
                                    } else if (ttl <= ((64 as f32) * 1.2) as u8) && (ttl >= ((64 as f32) * 0.8) as u8) {
                                        println!("OS of target calculated from PORT {} is: Linux/Microcontroller/Embedded Device", port);
                                    } else if (ttl <= ((128 as f32) * 1.2) as u8) && (ttl >= ((128 as f32) * 0.8) as u8) {
                                        println!("OS of target calculated from PORT {} is: Windows", port);
                                    } else if (ttl <= ((255 as f32) * 1.0) as u8) && (ttl >= ((255 as f32) * 0.8) as u8) {
                                        //                            ^
                                        //                            |
                                        //                       cuz its 8-bit
                                        println!("OS of target calculated from PORT {} is: MacOS", port);
                                    } else {
                                        println!("OS of target calculated from PORT {} is: Unknown", port);
                                    }
                                }  

                                break;
                            }
                        }
                    }
                }
            },
            Err(e) => {
                if e.kind() == ErrorKind::TimedOut || e.kind() == ErrorKind::WouldBlock {
                    println!("Port {} is FILTERED or CLOSED(Timedout)", port);

                    break;
                }
                else {
                    eprintln!("[-] Error occured while listening for response");
                    break;
                }
            }
        }
    }

    if services_scan == true {
        let address = format!("{}:{}", target_ipv4, port);

        let mut banner: String = String::new();

        match TcpStream::connect_timeout(&address.parse().unwrap(), Duration::from_secs(3)) {
            Ok(mut stream) => {
                stream.set_read_timeout(Some(Duration::from_secs(3))).unwrap();

                let mut buffer = [0; 1024];
                
                match stream.read(&mut buffer) {
                    Ok(bytes_read) => {
                        if bytes_read > 0 {
                            banner = String::from_utf8_lossy(&buffer[..bytes_read]).into();
                            println!("Version of software for PORT {}: {}", port, banner);
                        }
                        else {
                            println!("Couldn't get version of software for PORT {}", port);
                        }
                    }
                    Err(e) => {
                        println!("[-] Error while trying to read stream buffer: {}", e);
                    }
                };
            }
            Err(e) => {
                println!("[-] Error while opening socket to target for version scan: {}", e);
            }
        };
    }
}

pub fn scan_host(interface: &InterfaceInfo, target: String, ports: String, os_scan: bool, services_scan: bool) {
    //println!("{}, {}", os_scan, services_scan);

    let mut type_of_ports_input = 0;

    match ports.find(",") {
        Some(idx) => type_of_ports_input = 1,
        None => {}
    }

    match ports.find("-") {
        Some(idx) => type_of_ports_input = 2,
        None => {}
    }

    // firstly getting mac addr of that target for layer 2 of packet
    let target_ipv4: Ipv4Addr = target.parse().expect("[-] Failed to turn target as String into Ipv4");

    let packet: Vec<u8> = build_arp_packet(interface.mac, interface.ip, target_ipv4);

    let mac_addr = if let Some(mac_addr) = get_mac_addr(packet, interface, &target_ipv4){
        mac_addr
    }
    else {
        eprintln!("[-] Coudln't get targets MAC address");
        return;
    };

    match type_of_ports_input {
        0 => {
            //println!("[*] Scanning single port");
            scan_port(interface, &target, &mac_addr, ports.parse().unwrap(), services_scan, os_scan);
        },
        1 => {
            //println!("[*] Scanning list of ports");
            let itertator = ports.split(",");

            let ports_collected: Vec<&str> = itertator.collect();

            for port in ports_collected {
                scan_port(interface, &target, &mac_addr, port.parse().unwrap(), services_scan, os_scan);
            }
        },
        2 => {
            //println!("[*] Scanning range of ports");
            let itertator = ports.split("-");

            let ports_collected: Vec<&str> = itertator.collect();

            let down: u16 = ports_collected[0].parse().expect("[-] Invalid start port");
            let up: u16 = ports_collected[1].parse().expect("[-] Invalid end port");;

            for port in down..(up + 1) {
                scan_port(interface, &target, &mac_addr, port, services_scan, os_scan);
            }
        }
        _ => {}
    };
}