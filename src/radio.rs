use nrf51_pac::{Interrupt, NVIC, interrupt};

use crate::{mutex::Mutex, once_cell::OnceCell};


struct RadioStruct {
    radio: nrf51_pac::RADIO,
    timer: nrf51_pac::TIMER0
}

// Advertising packet data
static mut ADV_DATA: [u8; 31] = [
    0x02, 0x01, 0x06, // Flags (LE General Discoverable Mode)
    0x0B, 0x09,       // Complete Local Name (length and type)
    b'G', b'1', b'0', b'-', b'D', b'r', b'o', b'n', b'e', // "G10-Drone"
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Padding
];


static RADIO: Mutex<OnceCell<RadioStruct>> = Mutex::new(OnceCell::uninitialized());


/// Initialize BLE advertising.
pub fn initialize(
    received_radio: nrf51_pac::RADIO,
    received_timer: nrf51_pac::TIMER0,
    nvic: &mut NVIC,
) {
    RADIO.modify(|radio_struct| {
        radio_struct.initialize(RadioStruct {
            radio: received_radio,
            timer: received_timer
        });

        // Disable the radio while configuring
        radio_struct.radio.tasks_disable.write(|w| unsafe { w.bits(1) });

        // Configure radio for BLE 1Mbit mode
        radio_struct.radio.mode.write(|w| w.mode().ble_1mbit());
        radio_struct.radio.txpower.write(|w| w.txpower().pos4d_bm()); // Set TX power
        radio_struct.radio.frequency.write(|w| unsafe { w.bits(2) }); // Channel 37 (advertising channel)
        radio_struct.radio.pcnf0.write(|w| unsafe { 
            w.lflen().bits(8)
            .s0len().clear_bit()
            .s1len().bits(0) 
        });
        radio_struct.radio.base0.write(|w| unsafe { w.bits(0x12345678) }); // Example base address
        radio_struct.radio.prefix0.write(|w| unsafe { w.ap0().bits(0x8E).ap1().bits(0x89).ap2().bits(0xBE).ap3().bits(0xD6) });
        radio_struct.radio.crcinit.write(|w| unsafe { w.bits(0x555555) });
        radio_struct.radio.crcpoly.write(|w| unsafe { w.bits(0x00065B) });

        // Load advertising data into radio packet buffer
        radio_struct.radio.packetptr.write(|w| unsafe { w.bits(&ADV_DATA as *const u8 as u32) });

        // Configure timer for advertising interval (e.g., 100ms)
        radio_struct.timer.prescaler.write(|w| unsafe { w.prescaler().bits(0) });
        radio_struct.timer.cc[0].write(|w| unsafe { w.bits(16_000_000 / 100) }); // 100ms
        radio_struct.timer.intenset.write(|w| w.compare0().set_bit());
        radio_struct.timer.shorts.write(|w| w.compare0_clear().set_bit());
        radio_struct.timer.tasks_clear.write(|w| unsafe { w.bits(1) }); // Safety: Writing 1 to a task-clear register is allowed.

        radio_struct.timer.tasks_start.write(|w| unsafe { w.bits(1) });
        radio_struct.radio.tasks_txen.write(|w| unsafe { w.bits(1) });
        radio_struct.radio.tasks_start.write(|w| unsafe { w.bits(1) });
    });

    let scan_response: [u8; 31] = [0; 31]; // Empty scan response

    // Enable interrupts for radio events (optional)
    // unsafe {
    //     nvic.set_priority(nrf51_pac::Interrupt::RADIO, 1);
    //     nrf51_pac::NVIC::unmask(nrf51_pac::Interrupt::RADIO);
    // }

    // Configure timer interrupts
    // Safety: We are not using priority-based critical sections.
    // unsafe {
    //     nvic.set_priority(Interrupt::TIMER0, 1);
    //     NVIC::unpend(Interrupt::TIMER0);
    // }

    // Enable interrupts
    // Safety: We are not using mask-based critical sections.
    // unsafe {
    //     NVIC::unmask(Interrupt::TIMER0);
    // }
}


// #[interrupt]
// unsafe fn TIMER0() {
//     // Safety: interrupts are already turned off here, since we are inside an interrupt
//     let radio_struct = unsafe { RADIO.no_critical_section_lock_mut() };
//     radio_struct.radio.tasks_txen.write(|w| unsafe { w.bits(1) });
//     radio_struct.radio.tasks_start.write(|w| unsafe { w.bits(1) });
// }