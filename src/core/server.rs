use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};

use sdl3::render::FPoint;

use crate::core::shared::*;


pub struct Server {
	running: bool,
	clients: HashMap<u16, SocketAddr>,
	next_client_id: u16,
	mouses:  HashMap<u16, MouseCursor>,
	socket: UdpSocket
}

impl Server {
	fn update_or_add_cursor(&mut self, id: u16, cursor: MouseCursor) {
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

impl Session for Server {
	fn init(config: ProgramConfig) -> Option<Server> {
		let address = SocketAddr::new(config.ip, config.port);

		let socket = UdpSocket::bind(address).expect("Was unable to bind address to socket.");

		println!("Started Server:");
		println!("{}, {}, {}", config.mode, config.ip, config.port);

		Some(Server {
			running: true,
			clients: HashMap::new(),
			next_client_id: 0,
			mouses: HashMap::new(),
			socket
		})
	}

	fn update(&mut self) {
		let mut buf = [0u8; 1024];

		let (_length, origin_addr) = self.socket.recv_from(&mut buf).expect("Didn't receive data.");

		let packet: Packet = match Packet::from_bytes(buf.to_vec()) {
			Some(packet) => { packet },
			None => { return }
		};

		match PacketType::try_from(packet.packet_type) {
			Ok(PacketType::Acknowledge) => {
				self.clients.insert(self.next_client_id, origin_addr);

				let id = self.next_client_id.to_be_bytes();
				self.socket.send_to(&[0xFF, id[0], id[1]], origin_addr).expect(&format!("Was unable to send data to {origin_addr}"));

				self.mouses.insert(self.next_client_id, MouseCursor::new(FPoint::new(0.0, 0.0)));

				let packet_type = PacketType::Cursor as u8;
				let cursor_type = PacketCursor::Update as u8;

				for (id, mouse) in self.mouses.iter() {
					let cursor_id   = id.to_be_bytes().to_vec();

					let mut buf: Vec<u8> = Vec::new();

                    buf.push(packet_type);
                    buf.extend_from_slice(&(MouseCursor::payload_size() + 3).to_be_bytes());
                    buf.push(cursor_type);
                    buf.extend(cursor_id);
                    buf.extend(mouse.to_packet_payload());

					let _ = self.socket.send_to(buf.as_slice(), origin_addr);
				}

				self.next_client_id += 1;
			}
			Ok(PacketType::Cursor) => {
				let payload = &packet.payload;
				let cursor_type: u8 = payload[0];
				let origin_client_id: u16 = u16::from_be_bytes(payload[1..3].try_into().unwrap());

				let mouse_data = MouseCursor::from_packet_payload(&payload[3..3 + MouseCursor::payload_size() as usize].to_vec()).unwrap();

				match PacketCursor::try_from(cursor_type) {
                    Ok(PacketCursor::Update) => {
                    	self.update_or_add_cursor(origin_client_id, mouse_data);

                    	println!("{:?}", packet);

                    	let packet_data = packet.to_bytes();

                    	for (client_id, client_address) in self.clients.iter() {
                    		if *client_id == origin_client_id { continue }

                        	let _ = self.socket.send_to(packet_data.as_slice(), client_address);
                        }
                    }
                    Ok(PacketCursor::Delete) => {
                        let cursor_id = u16::from_be_bytes(payload[2..4].try_into().unwrap());
                        self.remove_cursor(cursor_id);
                    }
                    Err(error) => {
                        println!("Something went wrong on reading cursos packet. Error: {error}")
                    }
                };
			}
			Ok(PacketType::PacketError) |
			Err(_) => {}
		}
	}

	fn is_running(&self) -> bool {
		self.running
	}
}