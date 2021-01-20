#![no_main]
#![no_std]
// #![deny(missing_docs)]
#![deny(unsafe_code)]
#![deny(unstable_features)]

mod setup;
use core::convert::{From, Infallible};

use setup::*;

use cortex_m_rt::entry;
use stm32f4xx_hal as hal;

use crate::hal::spi::{self, Mode, NoMiso, Phase, Polarity, Spi};
use crate::hal::{delay::Delay, prelude::*, stm32, time::MegaHertz};

use embedded_hal::digital::v2::OutputPin;

use embedded_graphics::{
    fonts::{Font12x16, Text},
    prelude::*,
    text_style,
};

use epd_waveshare::{
    color::*,
    epd1in54b::{Display1in54b, EPD1in54b},
    graphics::Display,
    prelude::*,
};

#[derive(Debug)]
enum Error {
    SPI(spi::Error),
    Infallible,
}

impl defmt::Format for Error {
    fn format(&self, f: &mut defmt::Formatter) {
        match self {
            Error::SPI(e) => match e {
                spi::Error::Overrun => defmt::write!(f, "SPI overrun occurred"),
                spi::Error::ModeFault => defmt::write!(f, "SPI mode fault occurred"),
                spi::Error::Crc => defmt::write!(f, "SPI crc error occurred"),
                _ => defmt::write!(f, "SPI error occurred"),
            },
            Error::Infallible => unreachable!(),
        }
    }
}

impl From<spi::Error> for Error {
    fn from(err: spi::Error) -> Error {
        Error::SPI(err)
    }
}

impl From<Infallible> for Error {
    fn from(_: Infallible) -> Error {
        Error::Infallible
    }
}

fn blink<LED: OutputPin>(
    led: &mut LED,
    delay: &mut Delay,
    duration_ms: u32,
) -> Result<(), LED::Error> {
    led.set_low()?;
    delay.delay_ms(duration_ms);
    led.set_high()?;
    delay.delay_ms(duration_ms);
    Ok(())
}

fn heartbeat<LED: OutputPin>(led: &mut LED, delay: &mut Delay) -> Result<(), LED::Error> {
    for _ in 0..2 {
        blink(led, delay, 50)?;
    }
    delay.delay_ms(800_u32);
    Ok(())
}

#[entry]
fn main() -> ! {
    if let (Some(dp), Some(cp)) = (
        stm32::Peripherals::take(),
        cortex_m::peripheral::Peripherals::take(),
    ) {
        if let Err(e) = run(dp, cp) {
            defmt::error!("{:?}", e);
        }
    } else {
        defmt::error!("Could not initialiaze peripherals");
    }
    exit()
}

fn run(dp: stm32::Peripherals, cp: cortex_m::peripheral::Peripherals) -> Result<(), Error> {
    // Set up the system clock. We want to run at 48MHz for this one.
    let rcc = dp.RCC.constrain();
    let clocks = rcc.cfgr.sysclk(48.mhz()).freeze();

    // Create a delay abstraction based on SysTick
    let mut delay = Delay::new(cp.SYST, clocks);

    let gpioa = dp.GPIOA.split();
    let sck = gpioa.pa5.into_alternate_af5(); // -> SCLK
    let mosi = gpioa.pa7.into_alternate_af5(); // -> SDI
    let dc = gpioa.pa3.into_push_pull_output(); // -> D/C
    let cs = gpioa.pa4.into_push_pull_output(); // -> CS
    let rst = gpioa.pa2.into_push_pull_output(); // -> Reset
    let busy = gpioa.pa1.into_pull_down_input(); // -> Busy

    let mut spi = Spi::spi1(
        dp.SPI1,
        (sck, NoMiso, mosi),
        Mode {
            polarity: Polarity::IdleLow,
            phase: Phase::CaptureOnFirstTransition,
        },
        MegaHertz(4).into(),
        clocks,
    );

    // Setup the device
    let mut epd = EPD1in54b::new(&mut spi, cs, busy, dc, rst, &mut delay)?;

    // Setup the graphics
    let mut bw_display = Display1in54b::default();
    let mut red_display = Display1in54b::default();

    // Draw some text
    Text::new("Hello from Rust!", Point::new(1, 1))
        .into_styled(text_style!(
            font = Font12x16,
            text_color = Black,
            background_color = White
        ))
        .draw(&mut bw_display)?;

    // Draw some text in RED
    Text::new("Hello in color!", Point::new(1, 20))
        .into_styled(text_style!(
            font = Font12x16,
            text_color = Black,
            background_color = White
        ))
        .draw(&mut red_display)?;

    // Transfer the frame data to the epd and display it
    epd.update_color_frame(&mut spi, &bw_display.buffer(), &red_display.buffer())?;
    epd.display_frame(&mut spi)?;
    epd.sleep(&mut spi)?;

    defmt::info!("Message displayed!");

    let gpioc = dp.GPIOC.split();
    let mut led = gpioc.pc13.into_push_pull_output();
    let keybutton = gpioa.pa0.into_pull_up_input();
    loop {
        heartbeat(&mut led, &mut delay)?;

        // button pressed on low
        if keybutton.is_low()? {
            led.set_low()?;

            epd.wake_up(&mut spi, &mut delay)?;
            bw_display.clear_buffer(Color::Black);
            epd.update_and_display_frame(&mut spi, &bw_display.buffer())?;
            epd.sleep(&mut spi)?;
        }
    }
}
