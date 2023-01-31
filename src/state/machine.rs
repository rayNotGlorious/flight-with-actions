use prusst::{Pruss, IntcConfig, Evtout, Sysevt};
use serde_json::{Result, Value, Deserialize, Serialize};


pub struct StateMachine {

}

impl StateMachine {
    fn runSequence(jsonseq: &str) {
        let v: Value = serde_json::from_str(jsonseq);
        println!("Testing {} {}", v["sequence_code"], v["sequence"]);

        /**
        // WWWWWWWWWWWWWWW
        let mut pruss = match Pruss::new(&IntcConfig::new_populated()) {
            Ok(p) => p,
            Err(e) => match e {
                prusst::Error::AlreadyInstantiated
                    => panic!("You can't instantiate more than one `Pruss` object at a time."),
                prusst::Error::PermissionDenied
                    => panic!("You do not have permission to access the PRU subsystem: \
                               maybe you should run this program as root?"),
                prusst::Error::DeviceNotFound
                    => panic!("The PRU subsystem could not be found: are you sure the `uio_pruss` \
                               module is loaded and supported by your kernel?"),
                prusst::Error::OtherDeviceError
                    => panic!("An unidentified problem occured with the PRU subsystem: \
                               do you have a valid overlay loaded?")
            }
        };
        // Get a handle to an event out.
        let irq = pruss.intc.register_irq(Evtout::E0);
        // Open and load the PRU binary.
        let mut pru_binary = File::open("pruseq.bin").unwrap();
            // Run the PRU binary.
        unsafe { pruss.pru0.load_code(&mut pru_binary).unwrap().run(); }
        
        // Let us know when the LED is turned on.
        for i in 1..11 {
            // Wait for the PRU to trigger the event out.
            irq.wait();
            println!("Blink {}", i);

            // Clear the triggering interrupt and re-enable the host irq.
            pruss.intc.clear_sysevt(Sysevt::S19);
            pruss.intc.enable_host(Evtout::E0);
        }

        // Wait for completion of the PRU code.
        irq.wait();
        pruss.intc.clear_sysevt(Sysevt::S19);
        println!("Goodbye!");
        */
    }
}