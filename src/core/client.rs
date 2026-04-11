use std::collections::HashMap;
use std::net::{self};
use std::time::Duration;

use std::sync::mpsc;
use std::thread;

use sdl3::keyboard::Keycode;
use crate::core::shared::*;

pub struct Client {
    pub running: bool,
    pub sdl_context: sdl3::Sdl,
    pub video_subsystem: sdl3::VideoSubsystem,
    pub canvas: sdl3::render::Canvas<sdl3::video::Window>,
    pub event_pump: sdl3::EventPump,

	pub online_id: u16,
    pub receiver: mpsc::Receiver<Vec<u8>>,
	pub mouses: HashMap<u16, MouseCursor>,
}

impl Client {
	pub fn init(config: ProgramConfig) -> Option<Client> {
		let target_address = net::SocketAddr::new(config.ip, config.port);

		let socket = net::UdpSocket::bind("0.0.0.0:0").expect("Couldn't bind socket.");

		socket.connect(target_address)
		    .expect(&format!("Unable to connect to server ip : {target_address}"));
		socket.set_read_timeout(Some(Duration::from_secs(5)))
		    .expect("Couldn't set read timeout.");
		
		let mut buf = [0u8; 1024];

		println!("Client Started:");
		println!("{}, {}, {}", config.mode, config.ip, config.port);

		println!("Connecting to the server...");
		socket.send(&[0x1]).expect("Wasn't able to send confirmation data to the server.");

		match socket.recv(&mut buf) {
		    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock
		    || e.kind() == std::io::ErrorKind::TimedOut => {
		        println!("Timed out when waiting for server to respond.");
		        return None;
		    }
		    Err(e) => {
		        println!("{e}");
		        return None
		    }
		    _ => {}
		};

        let (transmitter, receiver) = mpsc::channel();
        thread::spawn(move || {
            loop {
                let mut buf = [0u8; 1024];
                match socket.recv(&mut buf) {
                    Ok(size) => {
                        transmitter.send(buf[0..size].to_vec()).unwrap();
                    },
                    Err(error) => {
                        println!("An error occurred while listening to the server: {error}");
                        break;
                    }
                }
            }
        });

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

        let event_pump = sdl_context.event_pump().unwrap();

		Some(Client {
            running: true,
            sdl_context,
            video_subsystem,
            canvas,
            event_pump,
			online_id: u16::from_be_bytes(buf[1..2].try_into().unwrap()),
            receiver: receiver,
            mouses: HashMap::new()
		})
	}

    pub fn update(&mut self) {
        match self.receiver.try_recv() {
            Ok(data) => {
                self.interpret_server_packet(data);
            },
            _ => {}
        }

        self.canvas.set_draw_color(sdl3::pixels::Color::BLACK);
        self.canvas.clear();

        for event in self.event_pump.poll_iter() {
            match event {
                sdl3::event::Event::Quit {..} |
                sdl3::event::Event::KeyDown {keycode: Some(Keycode::Escape), ..} => {
                    self.running = false;
                    return
                }
                _ => {}
            }
        }

        for (_id, cursor) in self.mouses.iter() {
            cursor.draw(&mut self.canvas);
        }

        let mouse_state = sdl3::mouse::MouseState::new(&self.event_pump);
        let client_cursor = MouseCursor::new((mouse_state.x(), mouse_state.y()));
        client_cursor.draw(&mut self.canvas);

        self.canvas.present();
    }

    fn interpret_server_packet(&mut self, data: Vec<u8>) {
        let packet = match Packet::from_bytes(data) {
            Some(data) => data,
            None => return
        };

        match PacketType::try_from(packet.packet_type) {
            Ok(PacketType::Cursor) => {
                let payload = packet.payload;

                match PacketCursor::try_from(payload[1]) {
                    Ok(PacketCursor::Update) => {
                        let cursor_id = u16::from_be_bytes(payload[2..3].try_into().unwrap());
                        let cursor = match MouseCursor::from_packet_payload(&payload[4..packet.payload_len as usize].to_vec()) {
                            Ok(cursor) => cursor,
                            Err(message) => {
                                println!("An error occured while reading a packet. type: {} error: {}", packet.packet_type, message);
                                return
                            }
                        };

                        self.update_or_add_cursor(cursor_id, cursor);
                    }
                    Ok(PacketCursor::Delete) => {
                        let cursor_id = u16::from_be_bytes(payload[2..3].try_into().unwrap());
                        self.remove_cursor(cursor_id);
                    },
                    Err(error) => {
                        println!("Something went wrong on reading cursos packet. Error: {error}")
                    }
                }
            },
            _ => {}
        }
    }

    fn update_or_add_cursor(&mut self, id: u16, cursor: MouseCursor) {
        if id == self.online_id { return }
        
        let mouse: &mut MouseCursor = match self.mouses.get_mut(&id) {
            Some(mouse) => mouse,
            None => {
                self.mouses.insert(id, cursor);
                return
            }
        };

        mouse.position = cursor.position;
        mouse.pressed  = cursor.pressed;
    }

    fn remove_cursor(&mut self, id: u16) {
        self.mouses.remove(&id);
    }
}

fn client_init(config: ProgramConfig) {
    let target_address = net::SocketAddr::new(config.ip, config.port);

    let socket = net::UdpSocket::bind("0.0.0.0:0").expect("Couldn't bind socket.");

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

    let window = video_subsystem.window("client", 800, 800)
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas();
    canvas.set_draw_color(sdl3::pixels::Color::BLACK);
    canvas.clear();
    canvas.present();

    let mut event_pump = sdl_context.event_pump().unwrap();

    let mut client_cursor = MouseCursor::new((0.0, 0.0));

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
                    client_cursor.position.x = mouse_state.x();
                    client_cursor.position.y = mouse_state.y();
                }
                _ => {}
            }
        }

        canvas.set_draw_color(sdl3::pixels::Color::RED);

        canvas.draw_rect(sdl3::render::FRect::new(
            client_cursor.position.x - 2.0,
            client_cursor.position.y - 2.0,
            4.0,
            4.0
        ));

        let _ = canvas.draw_point(client_cursor.position);

        canvas.present();
    }
}