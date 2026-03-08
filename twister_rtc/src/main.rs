#![no_std]
#![no_main]

use panic_halt as _;

use cortex_m_rt::entry;
use rp_pico::hal::{
    clocks::{init_clocks_and_plls, Clock},
    pac,
    usb::{UsbBus, UsbBusAllocator},
};
use usb_device::{
    class_prelude::*,
    prelude::*,
};
use usbd_serial::SerialPort;

#[entry]
fn main() -> ! {
    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();
    
    let system_clock = init_clocks_and_plls(
        rp_pico::XOSC_CRYSTAL_FREQ,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &core.CLOCKS,
        rp_pico::ClockConfiguration::default(),
    )
    .ok()
    .unwrap();

    let usb_bus = UsbBusAllocator::new(UsbBus::new(
        pac.USBCTRL_REGS,
        pac.USBCTRL_DPRAM,
        system_clock.usb_clock,
        true,
        &mut pac.RESETS,
    ));

    let mut serial = SerialPort::new(&usb_bus);
    let usb_dev = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x2e8a, 0x000a))
        .manufacturer("Twister")
        .product("Forensic RTC")
        .device_class(usb_class::CDC_CLASS)
        .build();

    let mut timer = rp_pico::hal::timer::Timer::new(pac.TIMER, &mut pac.RESETS);
    let start_cycles = timer.get_counter().cycles();

    loop {
        let mut buf = [0u8; 64];
        if serial.read(&mut buf).is_ok() {
            // TSREQ → 12-byte ns timestamp
            if buf.starts_with(b"TSREQ") {
                let cycles = timer.get_counter().cycles().wrapping_sub(start_cycles);
                let ns = ((cycles as u64 * 1000) / 150_000_000) as u64; // 150MHz → ns
                let ts_bytes = ns.to_le_bytes();
                serial.write(&ts_bytes).unwrap();
            }
        }
    }
}
