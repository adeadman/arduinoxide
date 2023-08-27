#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]

use core::cell;
use panic_halt as _;
use arduino_hal::prelude::*;

// Set up a cell counter to keep track of milliseconds since init
static MILLIS_COUNTER: avr_device::interrupt::Mutex<cell::Cell<u32>> =
    avr_device::interrupt::Mutex::new(cell::Cell::new(0));

const PRESCALER: u32 = 1024;  // 8, 64, 256 or 1024
const TIMER_COUNTS: u32 = 125;  // either 125 or 250
const MILLIS_INCREMENT: u32 = PRESCALER * TIMER_COUNTS / 16000;

fn init_millis(tc0: arduino_hal::pac::TC0) {
    // Configure the atmega328p timer for an interrupt for the above interval in CTC mode
    tc0.tccr0a.write(|w| w.wgm0().ctc());
    tc0.ocr0a.write(|w| w.bits(TIMER_COUNTS as u8));
    tc0.tccr0b.write(|w| match PRESCALER {
        8 => w.cs0().prescale_8(),
        64 => w.cs0().prescale_64(),
        256 => w.cs0().prescale_256(),
        1024 => w.cs0().prescale_1024(),
        _ => panic!(),
    });
    tc0.timsk0.write(|w| w.ocie0a().set_bit());

    // Reset the global millisecond counter
    avr_device::interrupt::free(|cs| {
        MILLIS_COUNTER.borrow(cs).set(0);
    });
}

// Interrupt handler - increment our cell counter
#[avr_device::interrupt(atmega328p)]
fn TIMER0_COMPA() {
    avr_device::interrupt::free(|cs| {
        let counter_cell = MILLIS_COUNTER.borrow(cs);
        let counter = counter_cell.get();
        counter_cell.set(counter + MILLIS_INCREMENT);
    })
}

// Read access to our global millisecond counter
fn millis() -> u32 {
    avr_device::interrupt::free(|cs| MILLIS_COUNTER.borrow(cs).get())
}

// -------------------------------------------------------------------

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);
    let mut serial = arduino_hal::default_serial!(dp, pins, 57600);

    init_millis(dp.TC0);

    // enable interrupts globally
    unsafe { avr_device::interrupt::enable() };

    let mut led_green = pins.d3.into_output();
    let mut led_amber = pins.d4.into_output();
    let mut led_red = pins.d5.into_output();
    let button = pins.d2.into_floating_input();
    let mut uno_led = pins.d13.into_output();
    ufmt::uwriteln!(&mut serial, "Hello from Arduino\r").void_unwrap();

    let mut last_uno_led_toggle_time = 0;

    loop {
        let current_millis = millis();
        if (last_uno_led_toggle_time + 1000) <= current_millis {
            last_uno_led_toggle_time = current_millis;
            uno_led.toggle();
        }
        if button.is_high() {
            ufmt::uwriteln!(&mut serial, "Button pressed! {} ms since boot.\r", millis()).void_unwrap();
            led_green.set_low();
            led_amber.set_low();
            led_red.set_high();
            arduino_hal::delay_ms(250);
            led_amber.set_high();
            led_red.set_low();
            arduino_hal::delay_ms(250);
        } else {
            led_green.set_high();
            led_amber.set_low();
            led_red.set_low();
        }
    }
}
