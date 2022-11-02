#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(future_join)]

use core::future::join;

use defmt::*;
use defmt_rtt as _;

use embassy_executor::Spawner;
use embassy_stm32::gpio::{Level, OutputOpenDrain, Pull, Speed};
use embassy_stm32::spi::{Config, Spi};
use embassy_stm32::time::Hertz;
use embassy_time::{Duration, Timer};

use panic_probe as _;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());

    info!("Initializing...");

    let mut spi = Spi::new_txonly(
        p.SPI1,
        p.PB3,
        p.PB5,
        p.DMA2_CH2,
        p.DMA2_CH3,
        Hertz(15_000_000),
        Config::default(),
    );

    let mut onboard_led = OutputOpenDrain::new(p.PC13, Level::High, Speed::Low, Pull::None);

    info!("Starting...");
    loop {
        {
            info!("On");
            onboard_led.set_low();

            let data: [u8; 4 + 4 * 9 + 4] = [
                // Start Frame.
                0x00, 0x00, 0x00, 0x00, // LED Frame.
                0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, // End Frame.
                0xff, 0xff, 0xff, 0xff,
            ];
            let spi_future = spi.write(&data);

            let timer_future = Timer::after(Duration::from_millis(500));

            let (write_result, _) = join!(spi_future, timer_future).await;
            write_result.unwrap();
        }

        {
            info!("Off");
            onboard_led.set_high();

            let data: [u8; 4 + 4 * 9 + 4] = [
                // Start Frame.
                0x00, 0x00, 0x00, 0x00, // LED Frame.
                0b11100000, 0x00, 0x00, 0x00, 0b11100000, 0x00, 0x00, 0x00, 0b11100000, 0x00, 0x00,
                0x00, 0b11100000, 0x00, 0x00, 0x00, 0b11100000, 0x00, 0x00, 0x00, 0b11100000, 0x00,
                0x00, 0x00, 0b11100000, 0x00, 0x00, 0x00, 0b11100000, 0x00, 0x00, 0x00, 0b11100000,
                0x00, 0x00, 0x00, // End Frame.
                0xff, 0xff, 0xff, 0xff,
            ];
            let spi_future = spi.write(&data);

            let timer_future = Timer::after(Duration::from_millis(500));

            let (write_result, _) = join!(spi_future, timer_future).await;
            write_result.unwrap();
        }
    }

    // for n in 0u32.. {
    //     let mut write: String<128> = String::new();
    //     let mut read = [0; 128];
    //     core::write!(&mut write, "Hello DMA World {n}!\r\n").unwrap();
    //     spi.transfer(&mut read[0..write.len()], write.as_bytes())
    //         .await
    //         .ok();
    //     info!("read via spi+dma: {}", from_utf8(&read).unwrap());
    // }
}
