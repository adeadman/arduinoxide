#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]

use arduino_hal::prelude::*;
use arduino_hal::simple_pwm::*;
use core::cell;
use panic_halt as _;

// Set up a cell counter to keep track of milliseconds since init
static MILLIS_COUNTER: avr_device::interrupt::Mutex<cell::Cell<u32>> =
    avr_device::interrupt::Mutex::new(cell::Cell::new(0));

const PRESCALER: u32 = 1024; // 8, 64, 256 or 1024
const TIMER_COUNTS: u32 = 125; // either 125 or 250
const MILLIS_INCREMENT: u32 = PRESCALER * TIMER_COUNTS / 16000;

const P13_BLINK_INTERVAL: u32 = 500; // milliseconds

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
    ufmt::uwriteln!(&mut serial, "Hello from Arduino\r").void_unwrap();

    init_millis(dp.TC0);

    // enable interrupts globally
    unsafe { avr_device::interrupt::enable() };

    let timer1 = Timer1Pwm::new(dp.TC1, Prescaler::Prescale64);
    let timer2 = Timer2Pwm::new(dp.TC2, Prescaler::Prescale64);

    let mut adc = arduino_hal::Adc::new(dp.ADC, Default::default());

    let mut red_pin = pins.d11.into_output().into_pwm(&timer2);
    let mut green_pin = pins.d9.into_output().into_pwm(&timer1);
    let mut blue_pin = pins.d10.into_output().into_pwm(&timer1);
    red_pin.enable();
    green_pin.enable();
    blue_pin.enable();
    let mut uno_led = pins.d13.into_output();

    let red_sensor = pins.a0.into_analog_input(&mut adc);
    let green_sensor = pins.a1.into_analog_input(&mut adc);
    let blue_sensor = pins.a2.into_analog_input(&mut adc);

    let mut last_uno_led_toggle_time = 0;

    loop {
        let current_millis = millis();
        if (last_uno_led_toggle_time + P13_BLINK_INTERVAL) <= current_millis {
            last_uno_led_toggle_time += P13_BLINK_INTERVAL;
            uno_led.toggle();
        }
        let red_value = red_sensor.analog_read(&mut adc);
        arduino_hal::delay_ms(5);
        let green_value = green_sensor.analog_read(&mut adc);
        arduino_hal::delay_ms(5);
        let blue_value = blue_sensor.analog_read(&mut adc);
        ufmt::uwriteln!(
            &mut serial,
            "Raw Sensor values (R,G,B): {}, {}, {}",
            red_value,
            green_value,
            blue_value,
        ).void_unwrap();

        red_pin.set_duty((red_value/5).try_into().unwrap());
        green_pin.set_duty((green_value/4).try_into().unwrap());
        blue_pin.set_duty((blue_value/5).try_into().unwrap());

        arduino_hal::delay_ms(10);
    }
}
