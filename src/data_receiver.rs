use std::net::{SocketAddr, UdpSocket};
use std::time::SystemTime;
use fs_protobuf_rust::compiled::mcfs::core;

use quick_protobuf::deserialize_from_slice;
use tracing::{trace, debug};

use crate::state;

// let sys_time = SystemTime::now();

pub struct DataReceiver {
    data_socket: UdpSocket,
    time: SystemTime,
}

impl DataReceiver {
    pub fn new() -> DataReceiver {
        let data_socket =
            UdpSocket::bind("0.0.0.0:4573").expect("Couldn't bind data_socket to address");
        data_socket
            .set_nonblocking(false)
            .expect("Couldn't set data socket to be non-blocking");
        let time = SystemTime::now();
        DataReceiver { data_socket, time}
    }

    pub fn receive(&mut self) -> Result<(usize, SocketAddr), std::io::Error> {
        let mut buf = [0; 1024];
        let (amt, src) = self.data_socket.recv_from(&mut buf)?;
        // println!("Received {} bytes from {} with delay {}", amt, src, self.time.elapsed().unwrap().as_millis());
        self.time = SystemTime::now();

        let deserialized: core::Message= deserialize_from_slice(&buf).unwrap();
        debug!("Received message from {}: {:?}", src, deserialized);
        // println!("Received message from {}: {:?}", src, deserialized);
        match deserialized.content {
            core::mod_Message::OneOfcontent::data(data) => {
                state::insert_sensor_data(data);
            }
            _ => {}
        }

        let _ = self.data_socket.send_to(&buf[..amt], "169.254.99.154:7201");
        Ok((amt, src))
    }
}
