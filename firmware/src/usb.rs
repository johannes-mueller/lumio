//! USB CDC ("USB UART") plumbing for the RP2040.
//!
//! This mirrors the original implementation from `main:src/interface.rs`
//! (commit `61b1d9e`): a CDC-class serial device with VID/PID
//! `0x16c0:0x27dd`, manufacturer `c3eo`, product `Lumio`, serial `001`.

use rp_pico::hal::usb::UsbBus;
use usb_device::bus::UsbBusAllocator;
use usb_device::prelude::{StringDescriptors, UsbDevice, UsbDeviceBuilder, UsbVidPid};
use usbd_serial::SerialPort;

/// USB device configuration (unchanged from the original firmware).
const USB_VID: u16 = 0x16c0;
const USB_PID: u16 = 0x27dd;
const USB_MANUFACTURER: &str = "c3eo";
const USB_PRODUCT: &str = "Lumio";
const USB_SERIAL: &str = "001";

pub type UsbSerial = SerialPort<'static, UsbBus>;
pub type UsbDev = UsbDevice<'static, UsbBus>;

/// Allocate the USB bus. Must run before the other peripherals are consumed
/// and before `init_clocks_and_plls` / `Pins::new` consume `RESETS`.
pub fn init(
    usbctrl_regs: rp_pico::hal::pac::USBCTRL_REGS,
    usbctrl_dpram: rp_pico::hal::pac::USBCTRL_DPRAM,
    usb_clock: rp_pico::hal::clocks::UsbClock,
    resets: &mut rp_pico::hal::pac::RESETS,
) -> (UsbSerial, UsbDev) {
    let usb_bus = cortex_m::singleton!(
        : UsbBusAllocator<UsbBus> =
            UsbBusAllocator::new(UsbBus::new(usbctrl_regs, usbctrl_dpram, usb_clock, true, resets))
    )
    .expect("Failed to allocate USB bus - singleton already exists");

    // Reborrow the singleton allocator as `&` for both consumers; the
    // `&mut` is only the base of these reborrows and is never used again.
    let usb_serial = SerialPort::new(&*usb_bus);

    let usb_dev = UsbDeviceBuilder::new(&*usb_bus, UsbVidPid(USB_VID, USB_PID))
        .strings(&[
            StringDescriptors::default()
                .manufacturer(USB_MANUFACTURER)
                .product(USB_PRODUCT)
                .serial_number(USB_SERIAL),
        ])
        .expect("Failed to set USB string descriptors")
        .device_class(2) // CDC class
        .build();

    (usb_serial, usb_dev)
}
