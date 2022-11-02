#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use defmt::*;
use defmt_rtt as _;

use panic_probe as _;

use embassy_executor::Spawner;
use embassy_stm32::exti::ExtiInput;
use embassy_stm32::gpio::{Input, Level, OutputOpenDrain, Pull, Speed};
use embassy_stm32::spi::{Config, Spi};
use embassy_stm32::time::Hertz;
use embassy_time::{Duration, Timer};
use futures::future::{select, Either};
use futures::pin_mut;

enum LightState {
    FadingOn,
    On,
    FadingOff,
    Off,
}

/// Generate an array of data for the SPI LED matrix.
/// Product page: https://www.adafruit.com/product/2351
const fn generate_led_data(brightness: u8) -> [u8; 4 + 4 * 9 + 4] {
    // The global brightness for all channels.
    const GLOBAL: u8 = 0b00001;
    // The header byte for each led.
    const HDR: u8 = 0b11100000 | GLOBAL;

    let data: [u8; 4 + 4 * 9 + 4] = [
        // Start Frame.
        0x00, 0x00, 0x00, 0x00, // LED Frame.
        HDR, brightness, brightness, brightness, HDR, brightness, brightness, brightness, HDR,
        brightness, brightness, brightness, HDR, brightness, brightness, brightness, HDR,
        brightness, brightness, brightness, HDR, brightness, brightness, brightness, HDR,
        brightness, brightness, brightness, HDR, brightness, brightness, brightness, HDR,
        brightness, brightness, brightness, // End Frame.
        0xff, 0xff, 0xff, 0xff,
    ];

    data
}

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

    let onboard_button = Input::new(p.PA0, Pull::Up);
    let mut onboard_button = ExtiInput::new(onboard_button, p.EXTI0);

    let motion_sensor = Input::new(p.PB1, Pull::None);
    let mut motion_sensor = ExtiInput::new(motion_sensor, p.EXTI1);

    let mut state = LightState::On;
    let mut brightness = 0xff;

    info!("Starting...");
    loop {
        match state {
            LightState::FadingOn => {
                info!("Fading On");

                // Fade the LED matrix on.
                while brightness != u8::MAX {
                    let data = generate_led_data(brightness);
                    let spi_future = spi.write(&data);
                    spi_future.await.unwrap();

                    brightness = brightness.saturating_add(1);

                    let timer_future = Timer::after(Duration::from_millis(2));
                    timer_future.await;
                }
                state = LightState::On;
            }
            LightState::On => {
                info!("On");
                onboard_led.set_low();

                let data = generate_led_data(0xff);
                let spi_future = spi.write(&data);
                spi_future.await.unwrap();

                let timer_future = Timer::after(Duration::from_secs(5 * 60));
                let motion_future = motion_sensor.wait_for_rising_edge();

                pin_mut!(timer_future, motion_future);
                match select(timer_future, motion_future).await {
                    Either::Left((_, _)) => {
                        trace!("Timer done first");
                        state = LightState::FadingOff;
                    }
                    Either::Right((_, _)) => {
                        trace!("Motion future done first");
                        state = LightState::On;
                    }
                }
            }
            LightState::FadingOff => {
                info!("FadingOff");

                // Fade the brightness of the light off, stopping if we detect motion.
                while brightness != 0 {
                    let data = generate_led_data(brightness);
                    let spi_future = spi.write(&data);
                    spi_future.await.unwrap();

                    let timer_future = Timer::after(Duration::from_millis(20));
                    let motion_future = motion_sensor.wait_for_rising_edge();

                    pin_mut!(timer_future, motion_future);
                    match select(timer_future, motion_future).await {
                        Either::Left((_, _)) => {
                            trace!("Timer future done first");
                            brightness = brightness.saturating_sub(1);
                        }
                        Either::Right((_, _)) => {
                            trace!("Motion future done first");
                            break;
                        }
                    }
                }
                if brightness == 0 {
                    state = LightState::Off;
                } else {
                    state = LightState::FadingOn;
                }
            }
            LightState::Off => {
                info!("Off");
                onboard_led.set_high();

                // Turn the LED matrix off.
                let data = generate_led_data(0x00);
                let spi_future = spi.write(&data);
                spi_future.await.unwrap();

                // Wait for either the button or the motion sensor to activate before switching states.
                let button_future = onboard_button.wait_for_falling_edge();
                let motion_future = motion_sensor.wait_for_rising_edge();

                pin_mut!(button_future, motion_future);
                match select(button_future, motion_future).await {
                    Either::Left((_, _)) => {
                        trace!("Button future done first");
                    }
                    Either::Right((_, _)) => {
                        trace!("Motion future done first")
                    }
                }

                // Begin fading the light on.
                state = LightState::FadingOn;
            }
        }
    }
}
