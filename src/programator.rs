use crate::programator_states::ProgramatorStates;
use alloc::vec::Vec;
use core::mem;
use esp_hal::uart::UartInterrupt;
use esp_hal::{Blocking, uart::Uart};

pub struct Programator {
    pub uart: Option<Uart<'static, Blocking>>,
    pub state: ProgramatorStates,
    pub bytes: Vec<u8>,
}

impl Programator {
    pub const fn new() -> Self {
        Self {
            uart: None,
            state: ProgramatorStates::Ready,
            bytes: Vec::new(),
        }
    }
    fn initialize(&mut self, first_byte: u8) -> Result<(), &str> {
        if !matches!(self.state, ProgramatorStates::Ready) {
            return Err("Cannot initialize programator");
        }

        self.state = ProgramatorStates::LoadSize {
            le_bytes: [first_byte, 0, 0, 0],
            step: 1,
        };

        Ok(())
    }

    fn load_size(&mut self, byte: u8, mut le_bytes: [u8; 4], mut step: u8) {
        le_bytes[step as usize] = byte;
        step += 1;

        if step >= 4 {
            let size = u32::from_le_bytes(le_bytes);
            self.state = ProgramatorStates::Loading { size };
        } else {
            self.state = ProgramatorStates::LoadSize { le_bytes, step };
        }
    }

    fn loading(&mut self, byte: u8, size: u32) {
        self.bytes.push(byte);

        if self.bytes.len() >= size as usize {
            self.state = ProgramatorStates::Loaded;
        }
    }

    pub fn load_byte(&mut self) -> Result<(), &str> {
        let uart = self.uart.as_mut().ok_or("No UART")?;
        if uart.interrupts().contains(UartInterrupt::RxBreakDetected) {
            esp_println::print!("\nBREAK");
            return Ok(());
        }

        let mut buf = [0u8; 1];
        let _ = uart.read(&mut buf);
        uart.clear_interrupts(UartInterrupt::RxBreakDetected | UartInterrupt::RxFifoFull);
        let byte = buf[0];

        match self.state {
            ProgramatorStates::Ready => self.initialize(byte)?,
            ProgramatorStates::LoadSize { le_bytes, step } => self.load_size(byte, le_bytes, step),
            ProgramatorStates::Loading { size } => self.loading(byte, size),
            ProgramatorStates::Loaded => {}
        }

        Ok(())
    }

    pub fn take_bytes(&mut self) -> Vec<u8> {
        let bytes = mem::replace(&mut self.bytes, Vec::new());
        self.state = ProgramatorStates::Ready;
        bytes
    }
}
