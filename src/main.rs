use std::env;
use std::net;
use std::net::IpAddr;
use std::process;
use std::str::FromStr;

enum ProgramMode {
    Server,
    Client
}

impl std::fmt::Display for ProgramMode {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ProgramMode::Client  => write!(formatter, "ClientMode"),
            ProgramMode::Server  => write!(formatter, "ServerMode"),
        }
    }
}

struct ProgramConfig {
    mode: ProgramMode,
    ip: net::IpAddr,
    port: u16
}


fn parse_mode(mode: &str) -> Result<ProgramMode, String> {
    match mode {
        "server" => Ok(ProgramMode::Server),
        "client" => Ok(ProgramMode::Client),
        _        => Err(format!("Invalid mode!"))
    }
}

fn parse_ip(ip: &str) -> Result<IpAddr, String> {
    IpAddr::from_str(ip).map_err(|err| format!("{}", err))
}


fn parse_config(args: Vec<String>) -> Result<ProgramConfig, String> {

    let mode: String = args.get(1)
        .ok_or("mode wasn't inputed")?
        .to_lowercase();

    let ip:   String = args.get(2)
        .ok_or(format!("IP address wasn't inputed"))?
        .to_string();

    let port: u16 = args.get(3)
        .map(|text_value| text_value.parse::<u16>().map_err(|_| format!("Invalid inputed port.")))
        .transpose()?
        .unwrap_or(8008);

    let config = ProgramConfig {
        mode: parse_mode(&mode)?,
        ip: parse_ip(&ip)?,
        port
    };

    Ok(config)
}

fn server_init(config: ProgramConfig) {
    let address = net::SocketAddr::new(config.ip, config.port);

    let socket = net::UdpSocket::bind(address).expect("Was unable to bind address to socket.");

    let mut buf = [0u8; 1024];

    println!("Started Server:");
    println!("{}, {}, {}", config.mode, config.ip, config.port);

    loop {
        let (length, origin_addr) = socket.recv_from(&mut buf).expect("Didn't receive data.");
        let message = String::from_utf8(buf[..length].to_vec()).unwrap();
    
        println!("{message}")
    }
    
}

fn client_init(config: ProgramConfig) {
    let target_address = net::SocketAddr::new(config.ip, config.port);

    let socket = net::UdpSocket::bind("0.0.0.0:0").unwrap();
    let _ = socket.set_broadcast(true);

    println!("client:");
    println!("{}, {}, {}", config.mode, config.ip, config.port);

    let _ = socket.send_to(b"And Jane! You're early!", target_address);
}

fn main() {
    let arguments: Vec<String> = env::args().collect();

    let config: ProgramConfig = match parse_config(arguments) {
        Ok(config) => config,
        Err(error) => {
            println!("{}", error);
            process::exit(1);
        }
    };

    match config.mode {
        ProgramMode::Server => server_init(config),
        ProgramMode::Client => client_init(config),
    };
}
