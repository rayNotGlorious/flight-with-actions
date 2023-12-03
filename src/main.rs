use std::thread;
use fc::data_receiver::DataReceiver;
use fc::flight_computer::FlightComputer;
use fc::sequences::libseq;
use fc::logger;
use fc::forwarding::ForwardingAgent;
use pyo3;

fn main() {

    pyo3::append_to_inittab!(libseq);

    let mut flight_computer = FlightComputer::new();
    let mut data_receiver = DataReceiver::new();
    let mut forwarding_agent = ForwardingAgent::new();

    let state_thread = thread::spawn(move || {
        tracing::subscriber::with_default(logger::file_logger("control").finish(), || {flight_computer.run() })
    });


    let data_thread = thread::spawn(move || {
        tracing::subscriber::with_default(logger::file_logger("data").finish(), || {loop {let _ = data_receiver.receive();}})
    });

    let forwarding_thread = thread::spawn(move || {
        tracing::subscriber::with_default(logger::file_logger("forwarding").finish(), || {loop {let _ = forwarding_agent.begin_forward();}})
    });

    data_thread.join().unwrap();
    state_thread.join().unwrap();
}
