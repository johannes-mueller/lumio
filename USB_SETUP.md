# USB Setup Guide for Lumio

This document explains how to properly set up USB functionality in the Lumio project, addressing the common `usb_bus` lifetime issue.

## The Issue

When trying to add USB support to embedded Rust projects using the RP2040, you'll encounter a lifetime issue with `UsbBusAllocator`. The allocator needs a `'static` lifetime because USB classes must reference it throughout the program's lifetime, but simply creating it as a local variable won't work.

## The Solution

Use the `cortex_m::singleton!` macro to create a static allocation. Here's how:

### Step 1: Add Dependencies

Add these to your `Cargo.toml`:

```toml
[dependencies]
usb-device = "0.2.9"
usbd-serial = "0.1.1"  # If you need serial functionality
```

### Step 2: Import Required Types

```rust
use rp_pico::hal::usb::UsbBus;
use usb_device::prelude::*;
use usbd_serial::SerialPort;  // If using serial
```

### Step 3: Create USB Bus with Static Lifetime

In your `Interface::new()` method, after initializing clocks:

```rust
// Create a static USB bus allocator using singleton!
let usb_bus = cortex_m::singleton!(
    : usb_device::bus::UsbBusAllocator<UsbBus> =
        usb_device::bus::UsbBusAllocator::new(UsbBus::new(
            pac.USBCTRL_REGS,
            pac.USBCTRL_DPRAM,
            clocks.usb_clock,
            true,
            &mut pac.RESETS,
        ))
).unwrap();
```

### Step 4: Create USB Device and Classes

```rust
// Create a USB serial port
let serial = SerialPort::new(&usb_bus);

// Create the USB device
// NOTE: The VID/PID 0x16c0:0x27dd are test values. For production devices,
// you must obtain a proper Vendor ID and Product ID from USB-IF or use
// your company's allocated VID with a unique PID.
let usb_dev = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x16c0, 0x27dd))
    .manufacturer("Your Company")
    .product("Lumio LED Controller")
    .serial_number("12345")
    .device_class(2) // CDC class
    .build();
```

### Step 5: Store in Interface Struct

Add these fields to your `Interface` struct:

```rust
pub struct Interface {
    // ... existing fields ...
    usb_serial: SerialPort<'static, UsbBus>,
    usb_dev: UsbDevice<'static, UsbBus>,
}
```

### Step 6: Poll USB in Your Main Loop

```rust
// In your main loop or update method
if self.usb_dev.poll(&mut [&mut self.usb_serial]) {
    // Handle USB events
}
```

## Why This Works

The `cortex_m::singleton!` macro creates a static variable with a `'static` lifetime, which is exactly what USB classes need. This is safe in embedded contexts because:

1. The USB bus is created once at startup
2. It lives for the entire program duration
3. Access is controlled through Rust's borrow checker
4. It's a common pattern in embedded Rust

## Alternative Approaches

### Using `static mut` (Less Safe)

```rust
static mut USB_BUS: Option<UsbBusAllocator<UsbBus>> = None;

unsafe {
    USB_BUS = Some(UsbBusAllocator::new(UsbBus::new(...)));
    let usb_bus = USB_BUS.as_ref().unwrap();
}
```

This approach is less preferred because it requires `unsafe` code and manual lifetime management.

### Using `OnceCell` or `lazy_static` (Limited `no_std` Support)

While standard library versions of these aren't available in bare-metal `no_std` environments, some alternatives like the `once_cell` crate's `race` feature can work in `no_std`. However, `cortex_m::singleton!` remains the recommended approach for embedded Rust as it's specifically designed for this use case and has zero runtime overhead.

## Common Errors

### Error: "temporary value dropped while borrowed"

This happens when you try to create the UsbBusAllocator without a static lifetime:

```rust
// ❌ This won't work
let usb_bus = UsbBusAllocator::new(UsbBus::new(...));
let serial = SerialPort::new(&usb_bus);  // Error: usb_bus doesn't live long enough
```

**Solution:** Use `cortex_m::singleton!` as shown above.

### Error: "cannot return value referencing local variable"

This happens when trying to return USB-related objects from a function without proper lifetime annotations.

**Solution:** Use the singleton pattern at the appropriate scope (usually in `main()` or a struct initialization).

## References

- [RP2040 HAL USB Example](https://github.com/rp-rs/rp-hal/blob/main/boards/rp-pico/examples/pico_usb_serial.rs)
- [usb-device Documentation](https://docs.rs/usb-device/)
- [cortex-m singleton! Documentation](https://docs.rs/cortex-m/latest/cortex_m/macro.singleton.html)
