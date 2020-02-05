#![no_std]
#![no_main]

extern crate panic_halt;

use feather_m0 as hal;
use rtfm;

use hal::{
    clock::GenericClockController,
    delay::Delay,
    gpio::{Output, Pa17, Pa5, Pb8, PushPull},
    pac::Peripherals,
    prelude::*,
};
use palantir::{feather_bus as bus, Palantir, SlaveAddresses};

use bus::UartBus;

const SLAVES: [u8; 1] = [0x2];

type ReceiveEnablePin = Pa5<Output<PushPull>>;
type StatusLEDPin = Pa17<Output<PushPull>>;
type ErrorLEDPin = Pb8<Output<PushPull>>;

#[rtfm::app(device = hal::pac)]
const APP: () = {
    struct Resources {
        palantir: Palantir<UartBus<ReceiveEnablePin>>,
        sercom0: hal::pac::SERCOM0,
        status_led: StatusLEDPin,
        error_led: ErrorLEDPin,
        delay: Delay,
    }
    #[init]
    fn init(cx: init::Context) -> init::LateResources {
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

        let receive_enable = pins.a4.into_push_pull_output(&mut pins.port);

        let uart = UartBus::easy_new(
            &mut clocks,
            peripherals.SERCOM0,
            &mut peripherals.PM,
            pins.d0,
            pins.d1,
            &mut pins.port,
            receive_enable,
        );

        let mut slaves: SlaveAddresses = SlaveAddresses::new();
        slaves.extend_from_slice(&SLAVES).unwrap();

        init::LateResources {
            palantir: Palantir::new_master(slaves, uart),
            sercom0: unsafe { Peripherals::steal().SERCOM0 },
            status_led: pins.d13.into_push_pull_output(&mut pins.port),
            error_led: pins.a1.into_push_pull_output(&mut pins.port),
            delay: Delay::new(cx.core.SYST, &mut clocks),
        }
    }

    #[idle(resources = [palantir, status_led, error_led, delay])]
    fn idle(cx: idle::Context) -> ! {
        let mut palantir = cx.resources.palantir;
        // Give a wee bit o' time to let slaves boot and enter discovery mode.
        // cx.resources.delay.delay_ms(1000u32);
        match palantir.lock(|p| p.discover_devices()) {
            Ok(_) => cx.resources.status_led.set_high().unwrap(),
            _ => cx.resources.error_led.set_high().unwrap(),
        };
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
