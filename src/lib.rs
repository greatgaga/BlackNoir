pub mod network;
pub mod cli;

use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;

pub fn start_session(interface: &network::InterfaceInfo) {
    //println!("{:?}", interface);

    let mut rl = rustyline::DefaultEditor::new().expect("[-] Error while creating session");

    loop {
        if let Some(user_input) = cli::cmd_input(&mut rl){
            let args: Vec<&str> = user_input.split_whitespace().collect();

            cli::parse(&args, &interface);
        }
        else {}
    }
}