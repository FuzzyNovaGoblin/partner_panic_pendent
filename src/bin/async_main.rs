#![no_std]
#![no_main]

use core::cell::RefCell;

use alloc::string::String;
use critical_section::Mutex;
use embassy_executor::Spawner;
use embassy_futures::select::{select, Either};
use embassy_time::{Duration, Ticker, Timer};
use esp_alloc as _;
use esp_backtrace as _;
use esp_backtrace as _;
use esp_hal::{
    clock::CpuClock,
    gpio::{Input, Io},
    handler,
    interrupt::InterruptConfigurable,
    ram,
    time::now,
};
use esp_println::println;
use esp_wifi::esp_now::{PeerInfo, BROADCAST_ADDRESS};
use log::info;

extern crate alloc;

const PENDENT_ADDRESS: [u8; 6] = [0x88u8, 0x88u8, 0x88u8, 0x88u8, 0x88u8, 0x88u8];
static BUTTON: Mutex<RefCell<Option<Input>>> = Mutex::new(RefCell::new(None));

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    // generator version: 0.2.2

    let peripherals = esp_hal::init(esp_hal::Config::default().with_cpu_clock(CpuClock::max()));

    esp_alloc::heap_allocator!(72 * 1024);

    esp_println::logger::init_logger_from_env();

    let timer0 = esp_hal::timer::systimer::SystemTimer::new(peripherals.SYSTIMER);
    esp_hal_embassy::init(timer0.alarm0);

    info!("Embassy initialized!");

    let timer1 = esp_hal::timer::timg::TimerGroup::new(peripherals.TIMG0);
    let esp_wifi_ctrl = esp_wifi::init(
        timer1.timer0,
        esp_hal::rng::Rng::new(peripherals.RNG),
        peripherals.RADIO_CLK,
    )
    .unwrap();

    let wifi = peripherals.WIFI;

    let mut esp_now = esp_wifi::esp_now::EspNow::new(&esp_wifi_ctrl, wifi).unwrap();

    println!("esp-now version {}", esp_now.version().unwrap());
    let mut ticker = Ticker::every(Duration::from_secs(1));

    // let mut last_pressed = todo!();

    // let mut flash = FlashStorage::new();

    // let flash_addr = 0x9000;

    spawner
        .spawn(button_listener(Input::new(
            peripherals.GPIO2,
            esp_hal::gpio::Pull::Down,
        )))
        .unwrap();

    loop {

        let res = select(ticker.next(), async {
            let r = esp_now.receive_async().await;
            println!("Received {:?}", r);
            println!(
                "data: {}",
                String::from_utf8(r.data().into()).unwrap_or("BAD DATA".into())
            );
            if r.info.dst_address == BROADCAST_ADDRESS {
                println!("connected to {:?}", r.info.src_address);
                if !esp_now.peer_exists(&r.info.src_address) {
                    esp_now
                        .add_peer(PeerInfo {
                            peer_address: r.info.src_address,
                            lmk: None,
                            channel: None,
                            encrypt: false,
                        })
                        .unwrap();
                }
                let status = esp_now.send_async(&r.info.src_address, b"Hello Peer").await;

                println!("Send hello to peer status: {:?}", status);
            }
        })
        .await;

        match res {
            Either::First(_) => {
                println!("Send");
                let status = esp_now.send_async(&BROADCAST_ADDRESS, b"0123456789").await;
                println!("Send broadcast status: {:?}", status);
            }
            Either::Second(_) => (),
        }
    }
}

#[embassy_executor::task]
async fn button_listener(button_pin: Input<'static>) {
    let mut button_pressed = false;
    let mut last_pressed = now();
    loop {
        if button_pin.is_high() && !button_pressed {
            button_pressed = true;
            last_pressed = now();
            println!("button just pressed");
        } else if button_pin.is_low() && button_pressed {
            let tim = now() - last_pressed;
            button_pressed = false;
            println!("button released, was pressed for {}", tim);
        }
        Timer::after(Duration::from_millis(1)).await;
    }
}
