use embedded_hal::{
    blocking::serial::{write::Default, Write},
    serial,
};
use hal::{
    clock::{GenericClockController, Sercom0CoreClock},
    gpio::{Floating, Input, Pa10, Pa11, PfC, Port},
    pac::{sercom0::USART, PM, SERCOM0},
    prelude::*,
    sercom::{PadPin, RxpoTxpo, Sercom0Pad2, Sercom0Pad3, UART0Padout},
    time::Hertz,
};
use palantir::Bus;

type Padout = UART0Padout<Sercom0Pad3<Pa11<PfC>>, Sercom0Pad2<Pa10<PfC>>, (), ()>;

pub struct UartBus {
    padout: Padout,
    sercom: SERCOM0,
}

impl UartBus {
    pub fn new<F: Into<Hertz>, T: Into<Padout>>(
        clock: &Sercom0CoreClock,
        freq: F,
        sercom: SERCOM0,
        pm: &mut PM,
        padout: T,
    ) -> UartBus
    where
        Padout: RxpoTxpo,
    {
        let padout = padout.into();

        pm.apbcmask.modify(|_, w| w.sercom0_().set_bit());

        // Lots of union fields which require unsafe access
        unsafe {
            // Reset
            sercom.usart().ctrla.modify(|_, w| w.swrst().set_bit());
            while sercom.usart().syncbusy.read().swrst().bit_is_set()
                || sercom.usart().ctrla.read().swrst().bit_is_set()
            {
                // wait for sync of CTRLA.SWRST
            }

            // Unsafe b/c of direct call to bits on rxpo/txpo
            sercom.usart().ctrla.modify(|_, w| {
                w.dord().set_bit();

                let (rxpo, txpo) = padout.rxpo_txpo();
                w.rxpo().bits(rxpo);
                w.txpo().bits(txpo);

                w.form().bits(0x00);
                w.sampr().bits(0x00); // 16x oversample fractional
                w.runstdby().set_bit(); // Run in standby
                w.form().bits(0); // 0 is no parity bits

                w.mode().usart_int_clk() // Internal clock mode
            });

            // Calculate value for BAUD register
            let sample_rate: u8 = 16;
            let fref = clock.freq().0;

            //          TODO: Support fractional BAUD mode
            //            let mul_ratio = (fref.0 * 1000) / (freq.into().0 * 16);
            //
            //            let baud = mul_ratio / 1000;
            //            let fp = ((mul_ratio - (baud*1000))*8)/1000;
            //
            //            sercom.usart().baud()_frac_mode.modify(|_, w| {
            //                w.baud().bits(baud as u16);
            //                w.fp().bits(fp as u8)
            //            });

            // Asynchronous arithmetic mode (Table 24-2 in datasheet)
            let baud = calculate_baud_value(freq.into().0, fref, sample_rate);

            sercom.usart().baud().modify(|_, w| w.baud().bits(baud));

            sercom.usart().ctrlb.modify(|_, w| {
                w.sbmode().clear_bit(); // 0 is one stop bit see sec 25.8.2
                w.chsize().bits(0x1); // 0x1 is 9 bit mode
                w.txen().set_bit();
                w.rxen().set_bit()
            });

            while sercom.usart().syncbusy.read().ctrlb().bit_is_set() {}

            sercom.usart().ctrla.modify(|_, w| w.enable().set_bit());
            // wait for sync of ENABLE
            while sercom.usart().syncbusy.read().enable().bit_is_set() {}
        }

        Self { padout, sercom }
    }

    pub fn easy_new(
        clocks: &mut GenericClockController,
        sercom0: SERCOM0,
        pm: &mut PM,
        rx: Pa11<Input<Floating>>,
        tx: Pa10<Input<Floating>>,
        port: &mut Port,
    ) -> UartBus {
        let gclk0 = clocks.gclk0();
        UartBus::new(
            &clocks.sercom0_core(&gclk0).unwrap(),
            1.mhz(),
            sercom0,
            pm,
            (rx.into_pad(port), tx.into_pad(port)),
        )
    }

    pub fn free(self) -> (Padout, SERCOM0) {
        (self.padout, self.sercom)
    }

    fn usart(&self) -> &USART {
        return &self.sercom.usart();
    }

    fn dre(&self) -> bool {
        self.usart().intflag.read().dre().bit_is_set()
    }
}

impl serial::Write<u8> for UartBus {
    type Error = ();

    fn write(&mut self, word: u8) -> nb::Result<(), Self::Error> {
        unsafe {
            if !self.dre() {
                return Err(nb::Error::WouldBlock);
            }

            self.sercom.usart().data.write(|w| w.bits(word as u16));
        }

        Ok(())
    }

    fn flush(&mut self) -> nb::Result<(), Self::Error> {
        // simply await DRE empty
        if !self.dre() {
            return Err(nb::Error::WouldBlock);
        }

        Ok(())
    }
}

impl serial::Read<u8> for UartBus {
    type Error = ();

    fn read(&mut self) -> nb::Result<u8, Self::Error> {
        let has_data = self.usart().intflag.read().rxc().bit_is_set();

        if !has_data {
            return Err(nb::Error::WouldBlock);
        }

        let data = self.usart().data.read().bits();

        Ok(data as u8)
    }
}

impl Default<u8> for UartBus {}

impl core::fmt::Write for UartBus {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.bwrite_all(s.as_bytes()).map_err(|_| core::fmt::Error)
    }
}

impl Bus for UartBus {
    type Error = ();
    // Ignore all sorts of errors for now kthx.
    fn send(&mut self, data: &[u16]) {
        for word in data.iter() {
            let _ = self.bwrite_all(&word.to_le_bytes());
        }
    }
    fn read(&mut self) -> nb::Result<u16, Self::Error> {
        let mut buf = [0u8; 2];
        for v in buf.iter_mut() {
            match <UartBus as serial::Read<u8>>::read(self) {
                Ok(data) => *v = data,
                Err(e) => return Err(e),
            }
        }
        Ok(u16::from_le_bytes(buf))
    }
}

const SHIFT: u8 = 32;

fn calculate_baud_value(baudrate: u32, clk_freq: u32, n_samples: u8) -> u16 {
    let sample_rate = (n_samples as u64 * baudrate as u64) << 32;
    let ratio = sample_rate / clk_freq as u64;
    let scale = (1u64 << SHIFT) - ratio;
    let baud_calculated = (65536u64 * scale) >> SHIFT;

    return baud_calculated as u16;
}
