use crate::network::{self, InterfaceInfo};

use std::io::{self, Write};
use clap::{Parser, Subcommand};
use std::process;

#[derive(Parser, Debug)]
#[command(no_binary_name = true, disable_help_subcommand = true)]
pub enum NoirTool {
    Discover {
        #[arg(short, long)]
        subnet: String
    },

    Scan {
        #[arg(short, long)]
        target: String,

        #[arg(short, long, num_args(0..=1), default_missing_value = "1-1024")]
        ports: String,

        #[arg(short, long, default_value_t = true)]
        os: bool,

        #[arg(short, long, default_value_t = true)]
        services: bool,

        #[arg(short, long, default_value_t = true)]
        all: bool,

        #[arg(short, long, default_value_t = false)]
        minimal_scan: bool
    },

    Exit,

    Help
}

pub fn prompt_user(prompt_text: &str) -> String {
    print!("{}", prompt_text);
    
    io::stdout().flush().expect("Failed to flush stdout buffer");

    let mut user_input = String::new();
    io::stdin()
        .read_line(&mut user_input)
        .expect("Failed to read user input");

    user_input.trim().to_string()
}

pub fn parse(args: &[&str], interface: &network::InterfaceInfo) {
    match NoirTool::try_parse_from(args) {
        Ok(tool) => {
            match tool {
                NoirTool::Discover{subnet} => {
                    println!("Scanning...");

                    network::scan_for_hosts(interface, subnet.to_string());
                }

                NoirTool::Scan{target, ports, os, services, all, minimal_scan} => {
                    if all == true {
                        network::scan_host(&interface, target, ports, true, true);
                    }
                    else if minimal_scan == true{
                        network::scan_host(&interface, target, ports, false, false);
                    }
                    else {
                        network::scan_host(&interface, target, ports, os, services);
                    }
                }

                NoirTool::Help => {
                    print_help();
                }
                
                NoirTool::Exit => {
                    println!("Shutting down session safely. Goodbye.");
                    process::exit(0);
                }
            }
        }

        Err(e) => {
            eprintln!("{}", e);
        }
    }
}

pub fn print_help() {
    println!("\nBlack Noir Command Center ── Help Menu");
    println!("──────────────────────────────────────────────────────────────────");
    println!("  COMMAND               USAGE EXAMPLES & DESCRIPTION");
    println!("  -------             --------------------------------------------");
    println!("  discover            discover --subnet 192.168.1 (or -s)");
    println!("                      Runs a tactical network discovery scan.");
    println!();
    println!("  scan                scan --target 192.168.1.50 (or -t) --os --services");
    println!("                      Scans specific hosts ports and analises.");
    println!();
    println!("  help                help");
    println!("                      Displays this control guide.");
    println!();
    println!("  exit                exit");
    println!("                      Safely terminates the active terminal session.");
    println!("──────────────────────────────────────────────────────────────────\n");
}