use pnet::datalink;
use pnet::ipnetwork::IpNetwork;
use pnet::util::MacAddr;
use std::net::Ipv4Addr;
use dict::{Dict, DictIface};

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