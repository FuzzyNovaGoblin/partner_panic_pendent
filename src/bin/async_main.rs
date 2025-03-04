#![no_std]
#![no_main]

use alloc::string::String;
use embassy_executor::Spawner;
use embassy_futures::select::{select, Either};
use embassy_time::{Duration, Ticker};
use esp_backtrace as _;
use esp_hal::clock::CpuClock;
use esp_println::println;
use esp_wifi::esp_now::PeerInfo;
use log::info;
use esp_alloc as _;
use esp_backtrace as _;


extern crate alloc;

const PENDENT_ADDRESS: [u8; 6] = [0x88u8, 0x88u8, 0x88u8, 0x88u8, 0x88u8, 0x88u8];

#[esp_hal_embassy::main]
async fn main(_spawner: Spawner) {
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
    let mut ticker = Ticker::every(Duration::from_secs(5));

    // let gpio = peripherals.GPIO;
    // gpio.enable();
    // let pin_reg = gpio.pin(2);
    // pin_reg.

    // Received ReceivedData { data: [72, 101, 108, 108, 111, 32, 80, 101, 101, 114], info: ReceiveInfo { src_address: [88, 207, 121, 233, 147, 52], dst_address: [96, 85, 249, 191, 255, 180], rx_control: RxControlInfo { rssi: 202, rate: 0, sig_mode: 0, mcs: 0, cwb: 0, smoothing: 0, not_sounding: 0, aggregation: 0, stbc: 0, fec_coding: 0, sgi: 0, ampdu_cnt: 0, channel: 1, secondary_channel: 0, timestamp: 2170329067, noise_floor: 160, ant: 0, sig_len: 53, rx_state: 0 } } }
    // Received ReceivedData { data: [48, 49, 50, 51, 52, 53, 54, 55, 56, 57], info: ReceiveInfo { src_address: [88, 207, 121, 233, 147, 52], dst_address: [255, 255, 255, 255, 255, 255], rx_control: RxControlInfo { rssi: 204, rate: 0, sig_mode: 0, mcs: 0, cwb: 0, smoothing: 0, not_sounding: 0, aggregation: 0, stbc: 0, fec_coding: 0, sgi: 0, ampdu_cnt: 0, channel: 1, secondary_channel: 0, timestamp: 2170838947, noise_floor: 160, ant: 0, sig_len: 53, rx_state: 0 } } }
    // Send hello to peer status: Ok(())
    loop {
        let res = select(ticker.next(), async {
            let r = esp_now.receive_async().await;
            println!("Received {:?}", r);
            println!(
                "data: {}",
                String::from_utf8(r.data().into()).unwrap_or("BAD DATA".into())
            );
            if r.info.dst_address == PENDENT_ADDRESS {
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
                let status = esp_now.send_async(&PENDENT_ADDRESS, b"0123456789").await;
                println!("Send broadcast status: {:?}", status)
            }
            Either::Second(_) => (),
        }
    }
}
