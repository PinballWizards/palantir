use crate::Bus;
use embedded_hal::{blocking::serial::write::Default, digital::v2::OutputPin, serial};
use feather_m0 as hal;
use hal::{
    clock::{GenericClockController, Sercom0CoreClock},
    gpio::{Floating, Input, Pa10, Pa11, PfC, Port},
    pac::{sercom0::USART, PM, SERCOM0},
    prelude::*,
    sercom::{PadPin, RxpoTxpo, Sercom0Pad2, Sercom0Pad3, UART0Padout},
    time::Hertz,
};

type Padout = UART0Padout<Sercom0Pad3<Pa11<PfC>>, Sercom0Pad2<Pa10<PfC>>, (), ()>;

pub struct UartBus<P: OutputPin> {
    padout: Padout,
    sercom: SERCOM0,
    transmit_enable: P,
}

impl<P: OutputPin> UartBus<P> {
    pub fn new<F: Into<Hertz>, T: Into<Padout>>(
        clock: &Sercom0CoreClock,
        freq: F,
        sercom: SERCOM0,
        pm: &mut PM,
        padout: T,
        mut transmit_enable: P,
    ) -> UartBus<P>
    where
        Padout: RxpoTxpo,
        <P as embedded_hal::digital::v2::OutputPin>::Error: core::fmt::Debug,
    {
        let padout = padout.into();
        transmit_enable.set_low().unwrap();

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

        Self {
            padout,
            sercom,
            transmit_enable,
        }
    }

    pub fn easy_new(
        clocks: &mut GenericClockController,
        sercom0: SERCOM0,
        pm: &mut PM,
        rx: Pa11<Input<Floating>>,
        tx: Pa10<Input<Floating>>,
        port: &mut Port,
        transmit_enable: P,
    ) -> UartBus<P>
    where
        <P as embedded_hal::digital::v2::OutputPin>::Error: core::fmt::Debug,
    {
        let gclk0 = clocks.gclk0();
        UartBus::new(
            &clocks.sercom0_core(&gclk0).unwrap(),
            9600.hz(),
            sercom0,
            pm,
            (rx.into_pad(port), tx.into_pad(port)),
            transmit_enable,
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

    pub fn enable_rxc_interrupt(&self) {
        self.usart().intenset.write(|w| w.rxc().set_bit());
    }

    pub fn enable_error_interrupt(&self) {
        self.usart().intenset.write(|w| w.error().set_bit());
    }
}

type Word = u16;

impl<P: OutputPin> serial::Write<Word> for UartBus<P> {
    type Error = ();

    fn write(&mut self, word: Word) -> nb::Result<(), Self::Error> {
        unsafe {
            if !self.dre() {
                return Err(nb::Error::WouldBlock);
            }

            self.sercom.usart().data.write(|w| w.bits(word));
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

impl<P: OutputPin> serial::Read<Word> for UartBus<P> {
    type Error = ();

    fn read(&mut self) -> nb::Result<Word, Self::Error> {
        let has_data = self.usart().intflag.read().rxc().bit_is_set();

        if !has_data {
            return Err(nb::Error::WouldBlock);
        }

        let data = self.usart().data.read().bits();

        Ok(data)
    }
}

impl<P: OutputPin> Default<Word> for UartBus<P> {}

impl<P: OutputPin> Bus for UartBus<P>
where
    <P as embedded_hal::digital::v2::OutputPin>::Error: core::fmt::Debug,
{
    type Error = ();
    // Ignore all sorts of errors for now kthx.
    fn send(&mut self, data: &[Word]) {
        self.transmit_enable.set_high().unwrap();
        for byte in data.iter() {
            let _ = nb::block!(self.write(*byte));
        }
        self.transmit_enable.set_low().unwrap()
    }
    fn read(&mut self) -> nb::Result<Word, Self::Error> {
        <UartBus<P> as serial::Read<Word>>::read(self)
    }
}

const SHIFT: u64 = 32;

fn calculate_baud_value(baudrate: u32, clk_freq: u32, n_samples: u8) -> u16 {
    let sample_rate = (n_samples as u64 * baudrate as u64) << 32;
    let ratio = sample_rate / clk_freq as u64;
    let scale = (1u64 << SHIFT) - ratio;
    let baud_calculated = (65536u64 * scale) >> SHIFT;

    return baud_calculated as u16;
}
