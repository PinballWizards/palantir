#![no_std]
#![no_main]

extern crate cortex_m;
extern crate cortex_m_semihosting;
extern crate feather_m0 as hal;
extern crate panic_halt;
extern crate rtfm;
#[macro_use]
extern crate nb;

use hal::{clock::GenericClockController, pac::Peripherals};
use palantir::{Palantir, SlaveAddresses};

mod bus;

use bus::UartBus;

const SLAVES: [u8; 1] = [0x1];

#[rtfm::app(device = hal::pac)]
const APP: () = {
    struct Resources {
        palantir: Palantir<UartBus>,
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

        let uart = UartBus::easy_new(
            &mut clocks,
            peripherals.SERCOM0,
            &mut peripherals.PM,
            pins.d0,
            pins.d1,
            &mut pins.port,
        );

        let mut slaves: SlaveAddresses = SlaveAddresses::new();
        slaves.extend_from_slice(&SLAVES).unwrap();

        init::LateResources {
            palantir: Palantir::new_master(slaves, uart),
            sercom0: unsafe { Peripherals::steal().SERCOM0 },
        }
    }

    #[idle(resources = [palantir])]
    fn idle(cx: idle::Context) -> ! {
        let mut palantir = cx.resources.palantir;
        palantir.lock(|p| {
            let _ = p.discover_devices();
        });
        loop {}
    }

    #[task(binds = SERCOM0, resources = [palantir, sercom0])]
    fn sercom0(cx: sercom0::Context) {
        let intflag = cx.resources.sercom0.usart_mut().intflag.read();
        if intflag.rxc().bit_is_set() {
            cx.resources.palantir.ingest();
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
