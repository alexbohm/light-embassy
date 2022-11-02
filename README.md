# Light
This is an implementation of a motion activated LED matrix.

This project uses the wonderful [embassy](https://github.com/embassy-rs/embassy) project to enable
async rust on the STM32F411CEU6 microcontroller.

Using the async framework, we can build a simple state machine that switches between a couple
states. Using this state machine, we switch between On, Off, and fade-between states. Having the
fade states allows us to eliminate the unexpected, fear-inducing blackness of other motion activated
lights.

## Materials
* https://www.adafruit.com/product/4877
* https://www.adafruit.com/product/2351
* Random PIR sensor

