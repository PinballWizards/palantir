#![no_std]
#![no_main]

extern crate cortex_m;
extern crate cortex_m_semihosting;
extern crate feather_m0 as hal;
extern crate panic_halt;
extern crate rtfm;

use hal::{clock::GenericClockController, pac::Peripherals, prelude::*};

const DEVICE_ADDRESS: u8 = 0x1;

#[rtfm::app(device = hal::pac)]
const APP: () = {
    struct Resources {
        sercom0: hal::pac::SERCOM0,
    }
    #[init]
    fn init(_: init::Context) -> init::LateResources {
        let mut peripherals = Peripherals::take().unwrap();
        let mut clocks = GenericClockController::with_external_32kosc(
            peripherals.GCLK,
            &mut peripherals.PM,
            &mut peripherals.SYSCTRL,
            &mut peripherals.NVMCTRL,
        );
        let mut pins = hal::Pins::new(peripherals.PORT);

        // Enable sercom0 receive complete interrupt and error interrupt
        peripherals.SERCOM0.usart_mut().intenset.write(|w| {
            w.rxc().set_bit();
            w.error().set_bit()
        });
        init::LateResources {
            sercom0: unsafe { Peripherals::steal().SERCOM0 },
        }
    }

    #[idle]
    fn idle(cx: idle::Context) -> ! {
        loop {}
    }

    #[task(binds = SERCOM0, resources = [sercom0])]
    fn sercom0(cx: sercom0::Context) {
        let intflag = cx.resources.sercom0.usart_mut().intflag.read();
        if intflag.rxc().bit_is_set() {
        } else if intflag.error().bit_is_set() {
            // Collision error detected, wait for NAK and resend
            cx.resources
                .sercom0
                .usart_mut()
                .intflag
                .write(|w| w.error().set_bit());
        }
    }

    extern "C" {
        fn SERCOM5();
    }
};
