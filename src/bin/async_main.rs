#![no_std]
#![no_main]

use core::cell::RefCell;
use critical_section::Mutex;
use embassy_executor::Spawner;
use embassy_futures::select::{select, Either};
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex,
    signal::Signal,
};
use embassy_time::{Duration, Ticker, Timer};
use embedded_storage::{ReadStorage, Storage};
use esp_alloc as _;
use esp_backtrace as _;
use esp_backtrace as _;
use esp_hal::{
    clock::CpuClock,
    gpio::{Input, Level, Output},
    time::now,
};
use esp_println::println;
use esp_storage::FlashStorage;
use esp_wifi::esp_now::{ReceivedData, BROADCAST_ADDRESS};
use log::info;
use partner_panic_pendent::buttonEvent::ButtonEvent;

extern crate alloc;

// static BUTTON_SIGNAL: Channel<Mutex, ButtonEvent, 3> = Signal::new();
static BUTTON_SIGNAL: Signal<CriticalSectionRawMutex, ButtonEvent> = Signal::new();

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    // init setup
    let peripherals = esp_hal::init(esp_hal::Config::default().with_cpu_clock(CpuClock::max()));
    esp_alloc::heap_allocator!(72 * 1024);
    esp_println::logger::init_logger_from_env();
    let timer0 = esp_hal::timer::systimer::SystemTimer::new(peripherals.SYSTIMER);
    esp_hal_embassy::init(timer0.alarm0);
    info!("Embassy initialized!");

    // pins
    let button_pin = Input::new(peripherals.GPIO2, esp_hal::gpio::Pull::Down);
    let mut vibrator_pin = Output::new(peripherals.GPIO3, Level::Low);

    // esp-now setup
    let timer1 = esp_hal::timer::timg::TimerGroup::new(peripherals.TIMG0);
    let esp_wifi_ctrl = esp_wifi::init(
        timer1.timer0,
        esp_hal::rng::Rng::new(peripherals.RNG),
        peripherals.RADIO_CLK,
    )
    .unwrap();
    let wifi = peripherals.WIFI;
    let mut esp_now = esp_wifi::esp_now::EspNow::new(&esp_wifi_ctrl, wifi).unwrap();
    info!("esp-now version {}", esp_now.version().unwrap());

    // storage setup
    // let mut flash = FlashStorage::new();
    // let flash_addr = 0x9000;
    // println!("Flash size = {}", flash.capacity());
    // let mut bytes = [0u8; 32];
    // flash.write(flash_addr, &mut bytes).unwrap();
    // flash.read(flash_addr, &mut bytes).unwrap();

    // println!("Read from {:x}:  {:02x?}", flash_addr, &bytes[..32]);

    // ---- tasks ----
    spawner.spawn(button_listener(button_pin)).unwrap();

    loop {
        {
            // let s = button_state.get_mut();
            // match s{
            //     ButtonEvent::None => ButtonEvent::None,
            //     ButtonEvent::Panic => {
            //         *s = ButtonEvent::None;
            //         ButtonEvent::Panic
            //     },
            // }
        };

        let res: Either<ButtonEvent, ReceivedData> =
            select(BUTTON_SIGNAL.wait(), esp_now.receive_async()).await;
        // let event = BUTTON_SIGNAL.wait().await;
        //         println!("button event {:?}", event);
        //         match event {
        //             ButtonEvent::None => (),
        //             ButtonEvent::Panic => {
        //                 let status = esp_now.send_async(&BROADCAST_ADDRESS, b"PANIC").await;
        //                 println!("status: {:?}", status);
        //             }
        //         }

        // let res: Either<(), ()> = select(ticker.next(), async {
        //     Timer::after_millis(1000).await
        //     // let r = esp_now.receive_async().await;
        //     // println!("Received {:?}", r);
        //     // println!(
        //     //     "data: {}",
        //     //     String::from_utf8(r.data().into()).unwrap_or("BAD DATA".into())
        //     // );
        //     // if r.info.dst_address == BROADCAST_ADDRESS {
        //     //     println!("connected to {:?}", r.info.src_address);
        //     //     if !esp_now.peer_exists(&r.info.src_address) {
        //     //         esp_now
        //     //             .add_peer(PeerInfo {
        //     //                 peer_address: r.info.src_address,
        //     //                 lmk: None,
        //     //                 channel: None,
        //     //                 encrypt: false,
        //     //             })
        //     //             .unwrap();
        //     //     }
        //     //     let status = esp_now.send_async(&r.info.src_address, b"Hello Peer").await;

        //     //     println!("Send hello to peer status: {:?}", status);
        //     // }
        // })
        // .await;

        match res {
            Either::First(event) => {
                info!("sending {:?}", event);
                match event {
                    ButtonEvent::None => (),
                    ButtonEvent::Panic => {
                        let status = esp_now.send_async(&BROADCAST_ADDRESS, &event.to_bstring()).await;
                        println!("status: {:?}", status);
                    }
                }
            }
            Either::Second(r) => {
                info!("received {:?}", r);
                match ButtonEvent::from_bstring(r.data()){
                    ButtonEvent::None => (),
                    ButtonEvent::Panic => run_vibrator(1000, &mut vibrator_pin).await,
                }
            }
        }
        Timer::after(Duration::from_millis(1)).await;
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
            BUTTON_SIGNAL.signal(ButtonEvent::Panic);
        }
        Timer::after(Duration::from_millis(1)).await;
    }
}

async fn run_vibrator(time: u64, vibrator_pin: &mut Output<'_>) {
    info!("running vibrator for {} ms", time);
    vibrator_pin.set_high();
    Timer::after(Duration::from_millis(time)).await;
    vibrator_pin.set_low();
}
