#![no_std]
#![no_main]

extern crate alloc;

use core::cell::RefCell;
use core::sync::atomic::AtomicBool;
use core::sync::atomic::Ordering;

use alloc::boxed::Box;
use critical_section::Mutex;
use esp_backtrace as _;
use esp_hal::Config;
use esp_hal::handler;
use esp_hal::init;
use esp_hal::main;
use esp_hal::ram;
use esp_hal::uart::Uart;
use esp_hal::uart::{Config as UartConfig, DataBits, Parity, RxConfig, StopBits, UartInterrupt};
use esp_println::println;
use kyrylscript::VM;

use crate::ks_std::ks_println::KsPrintln;
use crate::programator::Programator;
use crate::programator_states::ProgramatorStates;

mod ks_std;
mod programator;
mod programator_states;

esp_bootloader_esp_idf::esp_app_desc!();

static PROGRAMATOR: Mutex<RefCell<Programator>> = Mutex::new(RefCell::new(Programator::new()));
static READY: AtomicBool = AtomicBool::new(false);

#[main]
fn main() -> ! {
    let peripherals = init(Config::default());
    esp_alloc::heap_allocator!(size: 128 * 1024);

    let (tx_pin, rx_pin) = (peripherals.GPIO16, peripherals.GPIO17);

    let uart_config = UartConfig::default()
        .with_baudrate(115_200)
        .with_data_bits(DataBits::_8)
        .with_parity(Parity::None)
        .with_stop_bits(StopBits::_1)
        .with_rx(RxConfig::default().with_fifo_full_threshold(1));

    let mut uart = Uart::new(peripherals.UART1, uart_config)
        .expect("Failed to load UART")
        .with_tx(tx_pin)
        .with_rx(rx_pin);

    uart.set_interrupt_handler(get_new_program);

    critical_section::with(|cs| {
        uart.clear_interrupts(UartInterrupt::RxBreakDetected | UartInterrupt::RxFifoFull);
        uart.listen(UartInterrupt::RxBreakDetected | UartInterrupt::RxFifoFull);

        let mut programator = PROGRAMATOR.borrow_ref_mut(cs);
        programator.uart = Some(uart);
    });

    let mut vm = VM::from(Box::default());
    vm.add_native(Box::new(KsPrintln));

    loop {
        let uart_ready = READY.load(Ordering::Relaxed);
        if uart_ready {
            critical_section::with(|cs| {
                READY.store(false, Ordering::Relaxed);
                let mut programator = PROGRAMATOR.borrow(cs).borrow_mut();
                let bytes = programator.take_bytes();

                println!("{:?}", bytes);
                vm.reset(bytes.into_boxed_slice());
                vm.init();
            })
        } else {
            if !vm.is_empty() {
                let res = vm.step();
                if let Err(err) = res {
                    println!("KYRYLSCRIPT PANIC: {:?}", &err.message);
                    vm.reset(Box::default());
                }
            }
        }
    }
}

#[handler]
#[ram]
fn get_new_program() {
    critical_section::with(|cs| {
        let mut programator = PROGRAMATOR.borrow(cs).borrow_mut();
        let result = programator.load_byte();

        if let Err(error) = result {
            println!("ERROR: {}", error);
            return;
        }

        if let ProgramatorStates::Loaded = programator.state {
            READY.store(true, Ordering::Relaxed);
        }
    })
}
