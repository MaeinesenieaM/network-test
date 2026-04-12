use std::env;

use connection_test::core::shared::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<String> = env::args().collect();

    let mode: String = arguments.get(1)
        .ok_or("mode wasn't inputed")?
        .to_lowercase();
    
    let ip:   String = arguments.get(2)
        .ok_or(format!("IP address wasn't inputed"))?
        .to_string();
    
    let port: u16 = arguments.get(3)
        .map(|text_value| text_value.parse::<u16>().map_err(|_| format!("Invalid inputed port.")))
        .transpose()?
        .unwrap_or(8008);


    let config: ProgramConfig = ProgramConfig::try_from((mode, ip, port))?;

    let mut session: Box<dyn Session> = match config.mode {
        ProgramMode::Server => Box::new(connection_test::core::server::Server::init(config).unwrap()),
        ProgramMode::Client => Box::new(connection_test::core::client::Client::init(config).unwrap()),
    };

    while session.is_running() {
        session.update();
    }
    
    Ok(())
}
