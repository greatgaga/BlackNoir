use std::io::{self, Write};
use clap::{Parser, Subcommand};
use std::process;
use std::iter;

#[derive(Parser, Debug)]
#[command(no_binary_name = true, disable_help_subcommand = true)]
pub enum NoirTool {
    Scan {
        #[arg(short, long)]
        target: String
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

pub fn parse(args: &[&str]) {
    match NoirTool::try_parse_from(args) {
        Ok(tool) => {
            match tool {
                NoirTool::Scan{target} => {
                    println!("Scanning...");
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
    println!("  scan                scan --target 192.168.1.1  (or -t)");
    println!("                      Runs a tactical network footprint scan.");
    println!();
    println!("  inject              inject --payload \"stealth_ping\" (or -p)");
    println!("                      Transmits custom data packets into target.");
    println!();
    println!("  help                help");
    println!("                      Displays this control guide.");
    println!();
    println!("  exit                exit");
    println!("                      Safely terminates the active terminal session.");
    println!("──────────────────────────────────────────────────────────────────\n");
}