#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]

use arduino_hal::adc;
use arduino_hal::prelude::*;
use core::cell;
use panic_halt as _;
use ringbuffer::{RingBuffer, ConstGenericRingBuffer};

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

const BASELINE_TEMP: f32 = 22.0;
const SMOOTH_LAST: usize = 4;

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);
    let mut serial = arduino_hal::default_serial!(dp, pins, 57600);
    ufmt::uwriteln!(&mut serial, "Hello from Arduino\r").void_unwrap();

    let mut adc = arduino_hal::Adc::new(dp.ADC, Default::default());

    let (vbg, gnd, tmp) = (
        adc.read_blocking(&adc::channel::Vbg),
        adc.read_blocking(&adc::channel::Gnd),
        adc.read_blocking(&adc::channel::Temperature),
    );
    ufmt::uwriteln!(
        &mut serial,
        "Vbandgap: {}, Ground: {}, Temperature: {}",
        vbg,
        gnd,
        tmp
    )
    .void_unwrap();

    init_millis(dp.TC0);

    // enable interrupts globally
    unsafe { avr_device::interrupt::enable() };

    let mut led_one = pins.d3.into_output();
    let mut led_two = pins.d4.into_output();
    let mut led_three = pins.d5.into_output();
    let mut led_four = pins.d6.into_output();
    let mut uno_led = pins.d13.into_output();

    let sensor = pins.a0.into_analog_input(&mut adc);

    let mut last_uno_led_toggle_time = 0;

    let mut sensor_readings_buffer = ConstGenericRingBuffer::<_, SMOOTH_LAST>::new();

    loop {
        let current_millis = millis();
        if (last_uno_led_toggle_time + P13_BLINK_INTERVAL) <= current_millis {
            last_uno_led_toggle_time += P13_BLINK_INTERVAL;
            uno_led.toggle();
        }
        let sensor_value = sensor.analog_read(&mut adc);
        let sensor_voltage = sensor_value as f32 / 1024.0 * 5.0;
        let sensor_degrees_c = (sensor_voltage - 0.5) * 100.0;
        sensor_readings_buffer.push(sensor_value.into());
        ufmt::uwrite!(
            &mut serial,
            "Sensor: {}, Volts: ",
            sensor_value,
        ).void_unwrap();
        write_float(&mut serial, sensor_voltage, 3).void_unwrap();
        ufmt::uwrite!(
            &mut serial,
            ", Degrees C: "
        ).void_unwrap();
        write_float(&mut serial, sensor_degrees_c, 3).void_unwrap();
        ufmt::uwriteln!(&mut serial, "").void_unwrap();

        let smooth_average_value: u16 = sensor_readings_buffer.average();
        let smooth_average_voltage = smooth_average_value as f32 / 1024.0 * 5.0;
        let smooth_average_degrees_c = (smooth_average_voltage - 0.5) * 100.0;
        ufmt::uwrite!(&mut serial, "Average of last {} sensor readings: {} (Volts: ", sensor_readings_buffer.len(), smooth_average_value).void_unwrap();
        write_float(&mut serial, smooth_average_voltage, 3).void_unwrap();
        ufmt::uwrite!(&mut serial, ", Degrees C: ").void_unwrap();
        write_float(&mut serial, smooth_average_degrees_c, 3).void_unwrap();
        ufmt::uwriteln!(&mut serial, "").void_unwrap();

        let temperature_delta = smooth_average_degrees_c - BASELINE_TEMP;

        if temperature_delta <= 0.0 {
            led_one.set_low();
            led_two.set_low();
            led_three.set_low();
            led_four.set_low();
        } else if temperature_delta <= 1.5 {
            led_one.set_high();
            led_two.set_low();
            led_three.set_low();
            led_four.set_low();
        } else if temperature_delta <= 3.0 {
            led_one.set_high();
            led_two.set_high();
            led_three.set_low();
            led_four.set_low();
        } else if temperature_delta <= 4.5 {
            led_one.set_high();
            led_two.set_high();
            led_three.set_high();
            led_four.set_low();
        } else if temperature_delta > 4.5 {
            led_one.set_high();
            led_two.set_high();
            led_three.set_high();
            led_four.set_high();
        }

        arduino_hal::delay_ms(100);
    }
}


trait CollectionAverage<S> {
    fn average(&self) -> S;
}

impl<S, T, const CAP: usize> CollectionAverage<S> for ConstGenericRingBuffer::<T, CAP> 
where
    T: Eq + Copy,
    S: for<'a> core::iter::Sum<&'a T> + core::ops::Div<Output = S> + core::convert::From<u16>,
{
    fn average(&self) -> S {
        let divisor = S::try_from(self.len() as u16).unwrap();
        self.iter().sum::<S>() / divisor
    }
}

//fn write_float<W>(f: &mut ufmt::Formatter<'_, W>, value: f32, precision: u8) -> Result<(), W::Error>
//    where
//        W: ufmt::uWrite + ?Sized,
fn write_float<W>(f: &mut W, value: f32, precision: u8) -> Result<(), W::Error>
    where
        W: ufmt::uWrite + ?Sized,
{
    let (number, decimals) = float_to_int_f32(value, precision);
    ufmt::uwrite!(f, "{}", number)?;
    match precision {
        1 => ufmt::uwrite!(f, ".{}", decimals),
        2 if decimals >= 10 => ufmt::uwrite!(f, ".{}", decimals),
        2 if decimals < 10 => ufmt::uwrite!(f, ".0{}", decimals),
        3 if decimals >= 100 => ufmt::uwrite!(f, ".{}", decimals),
        3 if decimals < 100 => ufmt::uwrite!(f, ".0{}", decimals),
        3 if decimals < 10 => ufmt::uwrite!(f, ".00{}", decimals),
        4 if decimals >= 1000 => ufmt::uwrite!(f, ".{}", decimals),
        4 if decimals < 1000 && decimals >= 100 => ufmt::uwrite!(f, ".0{}", decimals),
        4 if decimals < 100 && decimals >= 10 => ufmt::uwrite!(f, ".00{}", decimals),
        4 if decimals < 10 => ufmt::uwrite!(f, ".000{}", decimals),
        5 if decimals >= 10000 => ufmt::uwrite!(f, ".{}", decimals),
        5 if decimals < 10000 && decimals >= 1000 => ufmt::uwrite!(f, ".0{}", decimals),
        5 if decimals < 1000 && decimals >= 100 => ufmt::uwrite!(f, ".00{}", decimals),
        5 if decimals < 100 && decimals >= 10 => ufmt::uwrite!(f, ".000{}", decimals),
        5 if decimals < 10 => ufmt::uwrite!(f, ".0000{}", decimals),
        _ => Ok(()),
    }
}

///Split the float into the integer and the fraction with the correct precision
fn float_to_int_f32(original: f32,precision: u8) -> (i32,u32) {
    let prec = match precision {
        1 => 10.0,
        2 => 100.0,
        3 => 1000.0,
        4 => 10000.0,
        5 => 100000.0,
        _ => 0.0,
    };
    let base = original as i32;
    let decimal = ((original - (base as f32)) * prec) as u32;
    (base,decimal)
}
