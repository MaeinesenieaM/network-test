use std::collections::HashMap;
use std::net::{self, SocketAddr, UdpSocket};
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

    pub socket: UdpSocket,

	pub online_id: u16,
    pub receiver: mpsc::Receiver<Vec<u8>>,
	pub mouses: HashMap<u16, MouseCursor>,

    pub client_mouse: MouseCursor
}

impl Client {
    fn interpret_server_packet(&mut self, data: Vec<u8>) {
        let packet = match Packet::from_bytes(data) {
            Some(data) => data,
            None => return
        };

        match PacketType::try_from(packet.packet_type) {
            Ok(PacketType::Cursor) => {
                let payload = packet.payload;

                match PacketCursor::try_from(payload[0]) {
                    Ok(PacketCursor::Update) => {
                        let cursor_id = u16::from_be_bytes(payload[1..3].try_into().unwrap());
                        let cursor = match MouseCursor::from_packet_payload(&payload[3..3 + MouseCursor::payload_size() as usize].to_vec()) {
                            Ok(cursor) => cursor,
                            Err(message) => {
                                println!("An error occured while reading a packet. type: {} error: {}", packet.packet_type, message);
                                return
                            }
                        };

                        self.update_or_add_cursor(cursor_id, cursor);
                    }
                    Ok(PacketCursor::Delete) => {
                        let cursor_id = u16::from_be_bytes(payload[1..3].try_into().unwrap());
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

impl Session for Client {
	fn init(config: ProgramConfig) -> Option<Client> {
		let target_address: SocketAddr = net::SocketAddr::new(config.ip, config.port);

		let socket: UdpSocket = net::UdpSocket::bind("0.0.0.0:0").expect("Couldn't bind socket.");

		socket.connect(target_address)
		    .expect(&format!("Unable to connect to server ip : {target_address}"));
		socket.set_read_timeout(Some(Duration::from_secs(5)))
		    .expect("Couldn't set read timeout.");
		
		let mut buf = [0u8; 1024];

		println!("Client Started:");
		println!("{}, {}, {}", config.mode, config.ip, config.port);

		println!("Connecting to the server...");

        let packet = Packet::from_bytes(vec!(PacketType::Acknowledge as u8, 0x00, 0x01, 0xff)).unwrap();

		socket.send(packet.to_bytes().as_slice()).expect("Wasn't able to send confirmation data to the server.");

        match socket.recv(&mut buf) {
            Err(_) => {
                println!("Timed out when waiting for server to respond.");
                return None
            }
            _ => {}
        }

        socket.set_read_timeout(None).unwrap();

        let listener_socket = socket.try_clone().expect("Couldn't clone socket for listener.");

        let (transmitter, receiver) = mpsc::channel();
        thread::spawn(move || {
            loop {
                let mut buf = [0u8; 1024];
                match listener_socket.recv(&mut buf) {
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

            socket,

			online_id: u16::from_be_bytes(buf[1..3].try_into().unwrap()),
            receiver: receiver,
            mouses: HashMap::new(),
            client_mouse: MouseCursor::new((0, 0))
		})
	}

    fn update(&mut self) {
        match self.receiver.try_recv() {
            Ok(data) => {
                self.interpret_server_packet(data);
            },
            _ => {}
        }

        self.canvas.set_draw_color(sdl3::pixels::Color::BLACK);
        self.canvas.clear();

        let mouse_state = sdl3::mouse::MouseState::new(&self.event_pump);

        self.client_mouse.position.x = mouse_state.x();
        self.client_mouse.position.y = mouse_state.y();

        self.client_mouse.pressed = mouse_state.left();

        for event in self.event_pump.poll_iter() {
            match event {
                sdl3::event::Event::Quit {..} |
                sdl3::event::Event::KeyDown {keycode: Some(Keycode::Escape), ..} => {
                    self.running = false;
                    return
                }
                sdl3::event::Event::MouseMotion {..} |
                sdl3::event::Event::MouseButtonDown {..} => {
                    let payload = self.client_mouse.to_packet_payload();

                    let packet_type = PacketType::Cursor as u8;
                    let cursor_type = PacketCursor::Update as u8;

                    let cursor_id   = self.online_id.to_be_bytes().to_vec();

                    let mut buf: Vec<u8> = Vec::new();

                    buf.push(packet_type);
                    buf.extend_from_slice(&(MouseCursor::payload_size() + 3).to_be_bytes());
                    buf.push(cursor_type);
                    buf.extend(cursor_id);
                    buf.extend(payload);

                    self.socket.send_to(buf.as_slice(), self.socket.peer_addr().unwrap()).unwrap();
                }
                _ => {}
            }
        }

        for (_id, cursor) in self.mouses.iter() {
            let _ = cursor.draw(&mut self.canvas);
        }
        
        let _ = self.client_mouse.draw(&mut self.canvas);

        self.canvas.present();
    }

    fn is_running(&self) -> bool {
        self.running
    }
}