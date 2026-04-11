use std::str::FromStr;
use std::net;

use sdl3::render;
use sdl3::pixels;

pub enum ProgramMode {
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

pub trait Draw {
    fn draw<T: render::RenderTarget>(&self, display: &mut render::Canvas<T>) -> Result<(), sdl3::Error>;
}

pub struct ProgramConfig {
    pub mode: ProgramMode,
    pub ip: net::IpAddr,
    pub port: u16
}

impl TryFrom<(String, String, String)> for ProgramConfig {
    type Error = String;

    fn try_from((mode, ip, port): (String, String, String)) -> Result<Self, Self::Error> {
        let mode = Self::parse_mode(&mode)?;
        let ip = Self::parse_ip(&ip)?;
        let port = Self::parse_port(&port)?;

        Ok(Self::new(mode, ip, port))
    }
}

impl TryFrom<(String, String, u16)> for ProgramConfig {
    type Error = String;

    fn try_from((mode, ip, port): (String, String, u16)) -> Result<Self, Self::Error> {
        let mode = Self::parse_mode(&mode)?;
        let ip = Self::parse_ip(&ip)?;

        Ok(Self::new(mode, ip, port))
    }
}

impl TryFrom<(String, net::IpAddr, u16)> for ProgramConfig {
    type Error = String;

    fn try_from((mode, ip, port): (String, net::IpAddr, u16)) -> Result<Self, Self::Error> {
        let mode = Self::parse_mode(&mode)?;

        Ok(Self::new(mode, ip, port))
    }
}

impl ProgramConfig {
    pub fn new(mode: ProgramMode, ip: net::IpAddr, port: u16) -> ProgramConfig {
        ProgramConfig {
            mode,
            ip,
            port
        }
    }

    fn parse_mode(mode: &str) -> Result<ProgramMode, String> {
        match mode {
            "server" => Ok(ProgramMode::Server),
            "client" => Ok(ProgramMode::Client),
            _        => Err(format!("Invalid mode!"))
        }
    }
    
    fn parse_ip(ip: &str) -> Result<net::IpAddr, String> {
        net::IpAddr::from_str(ip).map_err(|err| err.to_string())
    }

    fn parse_port(port: &str) -> Result<u16, String> {
        port.parse::<u16>().map_err(|_| format!("Invalid inputed port."))
    }
}

pub struct MouseCursor {
    pub position: render::FPoint,
    pub pressed:  bool,
    pub color:    pixels::Color,
}

impl MouseCursor {
    pub fn new(pos: impl Into<render::FPoint>) -> MouseCursor {
        let pos = pos.into();

        let r: u8 = fastrand::u8(30..210);
        let g: u8 = fastrand::u8(30..210);
        let b: u8 = fastrand::u8(30..210);

        MouseCursor {
            position: pos,
            pressed: false,
            color: pixels::Color::RGB(r, g, b)
        }
    }
}

impl Draw for MouseCursor {
    fn draw<T: render::RenderTarget>(&self, display: &mut render::Canvas<T>) -> Result<(), sdl3::Error> {
        let rect = render::FRect::new(
            self.position.x - 2.0,
            self.position.y - 2.0,
            4.0,
            4.0
        );

        let previous_draw_color = display.draw_color();
        display.set_draw_color(self.color);

        if self.pressed {
            display.fill_rect(rect)?;
        } else {
            display.draw_rect(rect)?;
        }

        display.set_draw_color(previous_draw_color);

        Ok(())
    }
}

#[derive(Debug)]
pub enum PacketType {
    Acknowledge  = 0xff,
    PacketError  = 0x00,
    Cursor       = 0x01,
}

impl TryFrom<u8> for PacketType {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0xff => Ok(PacketType::Acknowledge),
            0x00 => Ok(PacketType::PacketError),
            0x01 => Ok(PacketType::Cursor),
            unknown  => Err(unknown)
        }
    }
}

#[derive(Debug)]
pub enum PacketCursor {
    Update = 0x01,
    Delete = 0x02,
}

impl TryFrom<u8> for PacketCursor {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x01 => Ok(PacketCursor::Update),
            0x02 => Ok(PacketCursor::Delete),
            unknown  => Err(unknown)
        }
    }
}


#[derive(Debug)]
pub struct Packet {
    pub packet_type: u8,
    pub payload_len: u16,
    pub payload: Vec<u8>
}

impl Packet {
    pub fn from_bytes(bytes: Vec<u8>) -> Option<Self> {
        if bytes.len() < 3 {
            return None;
        }

        let packet_type: u8 = bytes[0];
        let payload_len: u16 = u16::from_be_bytes([bytes[1], bytes[2]]);
        let payload: Vec<u8> = bytes[3..3 + payload_len as usize].to_vec();

        Some(Packet {
            packet_type,
            payload_len,
            payload
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buffer = vec![self.packet_type];

        buffer.extend_from_slice(&self.payload_len.to_be_bytes());
        buffer.extend_from_slice(&self.payload);

        buffer
    }
}

#[derive(Debug)]
enum PacketError {
    TooShort { expected: usize, got: usize },
    InvalidData(String),
}

impl std::fmt::Display for PacketError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            PacketError::TooShort { expected, got } =>
                write!(formatter, "payload too short: expected {expected}, got {got}"),
            PacketError::InvalidData(msg) => write!(formatter, "invalid data: {msg}"),
        }
    }
}

impl std::error::Error for PacketError {}

pub trait PacketLoad {
    fn to_packet_payload(&self) -> Vec<u8>;

    fn from_packet_payload(payload: &Vec<u8>) -> Result<Self, PacketError> where Self: Sized;

    fn payload_size() -> usize;
}


impl PacketLoad for MouseCursor {
    fn to_packet_payload(&self) -> Vec<u8> {
        let mut buffer: Vec<u8> = vec![];
        
        buffer.extend_from_slice(&self.position.x.to_be_bytes());
        buffer.extend_from_slice(&self.position.y.to_be_bytes());

        buffer.push(self.pressed as u8);

        buffer.push(self.color.r as u8);
        buffer.push(self.color.g as u8);
        buffer.push(self.color.b as u8);
        buffer.push(self.color.a as u8);

        buffer
    }

    fn from_packet_payload(payload: &Vec<u8>) -> Result<MouseCursor, PacketError> {
        if payload.len() < Self::payload_size() {
            return Err(PacketError::TooShort { expected: Self::payload_size(), got: payload.len() })
        };

        let pos_x = f32::from_be_bytes(payload[0..4]
            .try_into()
            .map_err(|error: std::array::TryFromSliceError| PacketError::InvalidData(error.to_string()))?);
        let pos_y = f32::from_be_bytes(payload[4..8]
            .try_into()
            .map_err(|error: std::array::TryFromSliceError| PacketError::InvalidData(error.to_string()))?);

        let pressed = payload[8] != 0x00;

        let r = payload[9];
        let g = payload[10];
        let b = payload[11];
        let a = payload[12];

        Ok(MouseCursor {
            position: sdl3::render::FPoint::new(pos_x, pos_y),
            pressed: pressed,
            color: pixels::Color { r, g, b, a }
        })
    }

    fn payload_size() -> usize {13}
}

struct PacketReader {
    packet_type: u8,
    buffer: Vec<u8>
}

impl PacketReader {
    fn read<T: PacketLoad>(packet: &Packet) -> Option<T> {
        T::from_packet_payload(&packet.payload).ok()
    }
}