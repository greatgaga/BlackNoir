use std::process::Command;
use std::error::Error;
use pnet::datalink;
use pnet::ipnetwork::IpNetwork;
use std::io::{self, Write};
use dict::Dict;
use pnet::util::MacAddr;
use std::net::Ipv4Addr;
use dict::DictIface;

#[derive(Debug)]
struct DictEntry {
    name: String,
    mac: MacAddr,
    ip: Ipv4Addr
}

fn main() -> Result<(), Box<dyn Error>> {
    let _host = "192.168.1.169";

    let interfaces = datalink::interfaces();

    println!("Interfaces:");

    let mut interface_dict = Dict::<DictEntry>::new();

    for interface in interfaces {
        println!("Name: {}\nIndex: {}", interface.name, interface.index);
        println!("MAC: {:?}", interface.mac.unwrap());
        
        if let IpNetwork::V4(address) = interface.ips[0] {
            // adding interface to interface_dict for further use
            let info = DictEntry{
                name: interface.name.to_owned(), 
                mac: interface.mac.unwrap().to_owned(), 
                ip: address.ip().to_owned()
            };

            interface_dict.add(interface.index.to_string(), info);

            println!("Address: {:?}", address.ip());
        }
        else {
            println!("Address: Failed to get");
            return Ok(());
        }

        println!("------------------------");
    }

    /*
    println!("Dictionary Content:");
    for entry in interface_dict.iter() {
        println!("Key: {}, Value: {:?}", entry.key, entry.val);
    }
    */

    print!("Interface to use (index of interface needed): ");

    io::stdout().flush().expect("Failed to flush stdout buffer");

    let mut user_input = String::new();

    io::stdin().read_line(&mut user_input).expect("Failed to read user input");

    let real_input = user_input.trim();

    let interface = interface_dict.get(real_input).unwrap();

    //println!("{:?}", interface);

    

    Ok(())
}