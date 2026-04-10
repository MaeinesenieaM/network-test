use std::env;
use std::net;
use std::net::IpAddr;
use std::process;
use std::str::FromStr;
use std::time::{Duration, Instant};

use sdl3::keyboard::Keycode;

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

struct MouseCursor {
    position: (f32, f32)
}

fn server_init(config: ProgramConfig) {
    let address = net::SocketAddr::new(config.ip, config.port);

    let socket = net::UdpSocket::bind(address).expect("Was unable to bind address to socket.");

    let mut buf = [0u8; 1024];

    println!("Started Server:");
    println!("{}, {}, {}", config.mode, config.ip, config.port);

    loop {
        let (length, origin_addr) = socket.recv_from(&mut buf).expect("Didn't receive data.");

        match length {
            1 => {
                socket.send_to(&[0xFF], origin_addr).expect(&format!("Was unable to send data to {origin_addr}"));
            }
            _ => {
                let message = String::from_utf8(buf[..length].to_vec()).unwrap();
                println!("{message}")
            }
        }
    }
    
}

fn client_init(config: ProgramConfig) {
    let target_address = net::SocketAddr::new(config.ip, config.port);

    let socket = net::UdpSocket::bind("0.0.0.0:0").expect("Couldn't bing socket.");

    socket.connect(target_address)
        .expect(&format!("Unable to connect to server ip : {target_address}"));
    socket.set_read_timeout(Some(Duration::from_secs(5)))
        .expect("Couldn't set read timeout.");
    
    let mut buf = [0u8; 1024];
    let mut connected = false;

    println!("Client Started:");
    println!("{}, {}, {}", config.mode, config.ip, config.port);

    println!("Connecting to the server...");
    socket.send(&[0x1]).expect("Wasn't able to send confirmation data to the server.");

    match socket.recv(&mut buf) {
        Ok(_) => {
            if buf[0] == 0xFF {
                connected = true;
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock
        || e.kind() == std::io::ErrorKind::TimedOut => {
            println!("Timed out when waiting for server to respond.")
        }
        Err(e) => {
            println!("{e}")
        }
    }

    if connected == true {
        println!("Connected!");
        client_logic()
    }
}

fn client_logic() {
    let sdl_context = sdl3::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let window = video_subsystem.window("client", 200, 200)
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas();
    canvas.set_draw_color(sdl3::pixels::Color::BLACK);
    canvas.clear();
    canvas.present();

    let mut event_pump = sdl_context.event_pump().unwrap();

    let mut client_cursor = MouseCursor { position: (0.0, 0.0) };

    'running: loop {
        canvas.set_draw_color(sdl3::pixels::Color::BLACK);
        canvas.clear();

        let mouse_state = sdl3::mouse::MouseState::new(&event_pump);

        for event in event_pump.poll_iter() {
            match event {
                sdl3::event::Event::Quit {..} |
                sdl3::event::Event::KeyDown {keycode: Some(Keycode::Escape), ..} => {
                    break 'running
                }
                sdl3::event::Event::MouseMotion {..} => {
                    client_cursor.position.0 = mouse_state.x() + 50.0;
                    client_cursor.position.1 = mouse_state.y();
                }
                _ => {}
            }
        }

        canvas.set_draw_color(sdl3::pixels::Color::RED);
        let _ = canvas.draw_point(client_cursor.position);

        canvas.present();
    }
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
