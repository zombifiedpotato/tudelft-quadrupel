use alloc::format;
use nrf51_pac::{Interrupt, NVIC, interrupt, radio};

use crate::{debug_message::{debug_message_from_str, enqueue_debug_message, get_debug_message_queue}, mutex::Mutex, once_cell::OnceCell};


struct RadioStruct {
    radio: nrf51_pac::RADIO,
    timer: nrf51_pac::TIMER0,
    counter: u8,
    is_sender: bool,
}

// Static memory lo for radio DMA
static mut PCKT_DATA: [u8; 2] = [
    0x00, 0x00
];

static mut COPY_PCKT: [u8; 2] = [ 0x00, 0x00];


static RADIO: Mutex<OnceCell<RadioStruct>> = Mutex::new(OnceCell::uninitialized());


/// Initialize radio
pub fn initialize_send(
    received_radio: nrf51_pac::RADIO,
    received_timer: nrf51_pac::TIMER0,
    nvic: &mut NVIC,
) {
    RADIO.modify(|radio_struct| {
        radio_struct.initialize(RadioStruct {
            radio: received_radio,
            timer: received_timer,
            counter: 0,
            is_sender: true,
        });

        // Disable the radio while configuring
        radio_struct.radio.power.write(|w| unsafe { w.bits(1) });
        radio_struct.radio.tasks_disable.write(|w| unsafe { w.bits(1) });

        // Configure radio for nrf 1Mbit mode
        radio_struct.radio.mode.write(|w| w.mode().nrf_1mbit());
        radio_struct.radio.txpower.write(|w| w.txpower().pos4d_bm()); // Set TX power
        radio_struct.radio.frequency.write(|w| unsafe { w.bits(2) }); // 2402 MHz
        radio_struct.radio.pcnf0.write(|w| unsafe { 
            w.s0len().clear_bit()
            .lflen().bits(0)
            .s1len().bits(0) 
        });
        radio_struct.radio.pcnf1.write(|w| unsafe {
            w.whiteen().clear_bit() // Disable data whitening
            .balen().bits(3)
            .maxlen().bits(2) // Force to always send two bytes
            .statlen().bits(2)
        });

        // Address config
        radio_struct.radio.base0.write(|w| unsafe { w.bits(0x8E89BED6) }); // Base Address // Should be access address 0x8E89BED6
        radio_struct.radio.prefix0.write(|w| unsafe { 
            w.ap0().bits(0xD6)
        });
        radio_struct.radio.txaddress.write(|w| unsafe { w.bits(0) }); // Logical address 0 which is base address 0 and prefix 0

        // CRC Config
        radio_struct.radio.crccnf.write(|w| { w.len().disabled() }); // No CRC
        // radio_struct.radio.crcpoly.write(|w| unsafe { w.bits(0b1000000000000011001011011) }); // x24 + x10 + x9 + x6 + x4 + x3 + x + 1
        // radio_struct.radio.crcinit.write(|w| unsafe { w.bits(0x555555) });

        // Enable shortcuts for Ready -> Start and End -> Disable
        radio_struct.radio.shorts.write(|w| { w.ready_start().set_bit().end_disable().set_bit() });

        // Point radio to advertising packet
        radio_struct.radio.packetptr.write(|w| unsafe { w.bits(&PCKT_DATA as *const u8 as u32) });

        // Configure timer for sending interval
        radio_struct.timer.prescaler.write(|w| unsafe { w.prescaler().bits(0) });
        radio_struct.timer.cc[0].write(|w| unsafe { w.bits(16_000_000 / 50) }); // 100ms
        radio_struct.timer.intenset.write(|w| w.compare0().set_bit());
        radio_struct.timer.shorts.write(|w| w.compare0_clear().set_bit());
        radio_struct.timer.tasks_clear.write(|w| unsafe { w.bits(1) }); 

        // Enable radio sending
        radio_struct.timer.tasks_start.write(|w| unsafe { w.bits(1) });
        radio_struct.radio.tasks_txen.write(|w| unsafe { w.bits(1) });
        radio_struct.radio.tasks_start.write(|w| unsafe { w.bits(1) });
    });

    // let scan_response: [u8; 31] = [0; 31]; // Empty scan response

    // Enable interrupts for radio events 
    // unsafe {
    //     nvic.set_priority(Interrupt::RADIO, 1);
    //     NVIC::unpend(Interrupt::RADIO);
    //     NVIC::unmask(Interrupt::RADIO);
    // }

    // Configure timer interrupts
    // Safety: We are not using priority-based critical sections.
    unsafe {
        nvic.set_priority(Interrupt::TIMER0, 1);
        NVIC::unpend(Interrupt::TIMER0);
    }

    // Enable interrupts
    // Safety: We are not using mask-based critical sections.
    unsafe {
        NVIC::unmask(Interrupt::TIMER0);
    }
}



/// Initialize radio
pub fn initialize_read(
    received_radio: nrf51_pac::RADIO,
    received_timer: nrf51_pac::TIMER0,
    nvic: &mut NVIC,
) {
    RADIO.modify(|radio_struct| {
        radio_struct.initialize(RadioStruct {
            radio: received_radio,
            timer: received_timer,
            counter: 0,
            is_sender: false,
        });

        // Disable the radio while configuring
        radio_struct.radio.power.write(|w| unsafe { w.bits(1) });
        radio_struct.radio.tasks_disable.write(|w| unsafe { w.bits(1) });

        // Configure radio for nrf 1Mbit mode
        radio_struct.radio.mode.write(|w| w.mode().nrf_1mbit());
        radio_struct.radio.frequency.write(|w| unsafe { w.bits(2) }); // 2402 MHz
        radio_struct.radio.pcnf0.write(|w| unsafe { 
            w.s0len().clear_bit()
            .lflen().bits(0)
            .s1len().bits(0) 
        });
        radio_struct.radio.pcnf1.write(|w| unsafe {
            w.whiteen().clear_bit() // Disable data whitening
            .balen().bits(3)
            .maxlen().bits(2) // Force to always send two bytes
            .statlen().bits(2)
        });

        // Address config
        radio_struct.radio.base0.write(|w| unsafe { w.bits(0x8E89BED6) }); // Base Address // Should be access address 0x8E89BED6
        radio_struct.radio.prefix0.write(|w| unsafe { 
            w.ap0().bits(0xD6)
        });
        radio_struct.radio.rxaddresses.write(|w| unsafe { w.bits(0) }); // Logical address 0 which is base address 0 and prefix 0

        // CRC Config
        radio_struct.radio.crccnf.write(|w| { w.len().disabled() }); // No CRC
        // radio_struct.radio.crcpoly.write(|w| unsafe { w.bits(0b1000000000000011001011011) }); // x24 + x10 + x9 + x6 + x4 + x3 + x + 1
        // radio_struct.radio.crcinit.write(|w| unsafe { w.bits(0x555555) });

        // Enable shortcuts for Ready -> Start and End -> Disable
        radio_struct.radio.shorts.write(|w| { w.ready_start().set_bit().end_disable().set_bit() });

        // Point radio to advertising packet
        radio_struct.radio.packetptr.write(|w| unsafe { w.bits(&PCKT_DATA as *const u8 as u32) });

        // Configure timer for sending interval
        radio_struct.timer.prescaler.write(|w| unsafe { w.prescaler().bits(0) });
        radio_struct.timer.cc[0].write(|w| unsafe { w.bits(16_000_000 / 50) }); // 100ms
        radio_struct.timer.intenset.write(|w| w.compare0().set_bit());
        radio_struct.timer.shorts.write(|w| w.compare0_clear().set_bit());
        radio_struct.timer.tasks_clear.write(|w| unsafe { w.bits(1) }); 

        // Enable radio sending
        radio_struct.timer.tasks_start.write(|w| unsafe { w.bits(1) });
        radio_struct.radio.tasks_rxen.write(|w| unsafe { w.bits(1) });
        radio_struct.radio.tasks_start.write(|w| unsafe { w.bits(1) });
    });

    // let scan_response: [u8; 31] = [0; 31]; // Empty scan response

    // Enable interrupts for radio events 
    // unsafe {
    //     nvic.set_priority(Interrupt::RADIO, 1);
    //     NVIC::unpend(Interrupt::RADIO);
    //     NVIC::unmask(Interrupt::RADIO);
    // }

    // Configure timer interrupts
    // Safety: We are not using priority-based critical sections.
    unsafe {
        nvic.set_priority(Interrupt::TIMER0, 1);
        NVIC::unpend(Interrupt::TIMER0);
    }

    // Enable interrupts
    // Safety: We are not using mask-based critical sections.
    unsafe {
        NVIC::unmask(Interrupt::TIMER0);
    }
}


pub fn read_state() {
    let mut state = 0;
    RADIO.modify(|radio| {
        state = radio.radio.state.read().bits();
    });
    enqueue_debug_message(debug_message_from_str(format!("Radio State: {}", state).as_str()));
    unsafe{
        enqueue_debug_message(debug_message_from_str(format!("Radio Packet: {}", COPY_PCKT[0]).as_str()));
    }
}

// #[interrupt]
// unsafe fn RADIO() {
//     // I dont fire for some reason stupid chip
//     // let message_queue = get_debug_message_queue();
//     // message_queue.push_back(debug_message_from_str("Radio Event (Interrupt)"));

//     let radio_struct = unsafe { RADIO.no_critical_section_lock_mut() };
//     if radio_struct.radio.events_ready.read().bits() != 0 {
//         // message_queue.push_back(debug_message_from_str("Radio Event (Interrupt): READY"));
//         radio_struct.radio.events_ready.reset();
//     }
//     if radio_struct.radio.events_end.read().bits() != 0 {
//         // message_queue.push_back(debug_message_from_str("Radio Event (Interrupt): END"));
//         radio_struct.radio.events_end.reset();
//     }
//     if radio_struct.radio.events_disabled.read().bits() != 0 {
//         // message_queue.push_back(debug_message_from_str("Radio Event (Interrupt): DISABLED"));
//         radio_struct.radio.events_disabled.reset();
//     }
// }

#[interrupt]
unsafe fn TIMER0() {
    // Safety: interrupts are already turned off here, since we are inside an interrupt

    // Cannot enqueue during interrupt because of panic. But this interrupt definetly fires.
    // let message_queue = get_debug_message_queue();
    // message_queue.push_back(debug_message_from_str("Timer Event (Interrupt)"));

    let radio_struct = unsafe { RADIO.no_critical_section_lock_mut() };
    if radio_struct.timer.events_compare[0].read().bits() != 0 {
        radio_struct.timer.events_compare[0].reset();

        if radio_struct.radio.state.read().bits() == 0 { // If radio is disabled, safe to write to shared DMA space
            COPY_PCKT = PCKT_DATA;
            PCKT_DATA = [radio_struct.counter, 0x00];
            radio_struct.counter = radio_struct.counter + 1;
        }
        if radio_struct.is_sender {
            radio_struct.radio.tasks_txen.write(|w| unsafe { w.bits(1) });
        } else {
            radio_struct.radio.tasks_rxen.write(|w| unsafe { w.bits(1)});
        }
        radio_struct.radio.tasks_start.write(|w| unsafe { w.bits(1)});
    }
}