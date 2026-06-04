pub mod network;
pub mod cli;

pub fn start_session(interface: &network::InterfaceInfo) {
    loop {
        let user_input = cli::prompt_user(">> ");

        let args: Vec<&str> = user_input.split_whitespace().collect();

        cli::parse(&args);
    }
}