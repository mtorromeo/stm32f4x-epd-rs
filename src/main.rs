#![no_main]
#![no_std]
// #![deny(missing_docs)]
#![deny(unsafe_code)]
#![deny(unstable_features)]

mod setup;
use setup::*;

use cortex_m_rt::entry;
use stm32f4xx_hal as hal;

use crate::hal::spi::{Mode, NoMiso, Phase, Polarity, Spi};
use crate::hal::{delay::Delay, prelude::*, stm32, time::MegaHertz};

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

#[entry]
fn main() -> ! {
    if let (Some(dp), Some(cp)) = (
        stm32::Peripherals::take(),
        cortex_m::peripheral::Peripherals::take(),
    ) {
        // Set up the LEDs
        let gpioc = dp.GPIOC.split();
        let mut led = gpioc.pc13.into_push_pull_output();

        let gpioa = dp.GPIOA.split();

        // Set up the system clock. We want to run at 48MHz for this one.
        let rcc = dp.RCC.constrain();
        let clocks = rcc.cfgr.sysclk(48.mhz()).freeze();

        // Create a delay abstraction based on SysTick
        let mut delay = Delay::new(cp.SYST, clocks);

        let sck = gpioa.pa5.into_alternate_af5();     // -> SCLK
        let mosi = gpioa.pa7.into_alternate_af5();    // -> SDI
        let dc = gpioa.pa3.into_push_pull_output();   // -> D/C
        let cs = gpioa.pa4.into_push_pull_output();   // -> CS
        let rst = gpioa.pa2.into_push_pull_output();  // -> Reset
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

        // Setup the epd
        if let Ok(mut epd) = EPD1in54b::new(&mut spi, cs, busy, dc, rst, &mut delay) {
            // Setup the graphics
            let mut display = Display1in54b::default();

            // Draw some text
            Text::new("Hello from Rust!", Point::new(1, 1))
                .into_styled(text_style!(
                    font = Font12x16,
                    text_color = Black,
                    background_color = White
                ))
                .draw(&mut display)
                .unwrap();

            // Transfer the frame data to the epd and display it
            epd.update_and_display_frame(&mut spi, &display.buffer())
                .unwrap();
            epd.sleep(&mut spi).unwrap();

            defmt::info!("Message displayed!");
        }

        loop {
            led.set_low().unwrap();
            delay.delay_ms(200_u32);
            led.set_high().unwrap();
            delay.delay_ms(200_u32);
        }
    }

    exit()
}
