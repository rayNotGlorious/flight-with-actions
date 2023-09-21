pub mod flight_computer;
pub mod discovery;
use flight_computer::state;

fn main() {
    let mut fc_state = state::State::Init;
    let mut data = state::Data::new();
    data.data_socket.set_nonblocking(true).expect("set_nonblocking call failed");
    loop {
        fc_state = fc_state.next(&mut data);
    }
}




