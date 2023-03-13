pub mod software_system_check;
pub mod software_system_check_err;
pub mod read_local_configs;
pub mod connect_to_controller;
pub mod spawn_cmd_receiver;

use software_system_check::SoftwareSystemCheck;
use software_system_check_err::SoftwareSystemCheckErr;
use read_local_configs::ReadLocalConfigs;
use connect_to_controller::ConnectToController;
use spawn_cmd_receiver::SpawnCmdReceiver;

#[derive(PartialEq, Debug)]
pub enum State {
    SoftwareSystemCheckState(SoftwareSystemCheck),
    SoftwareSystemCheckErrState(SoftwareSystemCheckErr),
    ReadLocalConfigsState(ReadLocalConfigs),
    ConnectToControllerState(ConnectToController),
    SpawnCmdReceiverState(SpawnCmdReceiver),
    UnknownState,
}

pub trait Stateful {
    fn next(self) -> State;
}