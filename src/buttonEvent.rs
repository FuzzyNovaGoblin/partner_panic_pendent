use esp_println::dbg;
use log::info;

#[derive(Debug)]
pub enum ButtonEvent {
    None,
    Panic,
}

impl ButtonEvent {
    pub fn to_bstring(&self) -> &[u8] {
        match self {
            ButtonEvent::None => &[0u8; 1],
            ButtonEvent::Panic => b"PANIC",
        }
    }

    pub fn from_bstring(bytes: &[u8]) -> ButtonEvent {
        info!("processing {:?}",bytes);
        match bytes {
            b"PANIC" => dbg!(ButtonEvent::Panic),
            _ => dbg!(ButtonEvent::None),
        }
    }
}
