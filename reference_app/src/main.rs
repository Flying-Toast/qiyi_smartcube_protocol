mod crc;
mod cube;
mod cubestate;
mod messages;

use crate::cube::Cube;
use btleplug::api::{bleuuid::uuid_from_u16, Central, CentralEvent, Manager as _, Peripheral as _};
use btleplug::platform::Manager;
use futures_util::StreamExt;
use std::io::{self, Write};

async fn async_main() {
    let manager = Manager::new().await.unwrap();
    let central = manager
        .adapters()
        .await
        .unwrap()
        .into_iter()
        .nth(0)
        .unwrap();

    println!("Searching for cube...");
    let mut events = central.events().await.unwrap();
    central.start_scan(Default::default()).await.unwrap();

    let cube_perip;
    loop {
        if let CentralEvent::DeviceDiscovered(id) = events.next().await.unwrap() {
            let peripheral = central.peripheral(&id).await.unwrap();
            if peripheral
                .properties()
                .await
                .unwrap()
                .unwrap()
                .local_name
                .iter()
                .any(|name| name.starts_with("QY-QYSC"))
            {
                cube_perip = peripheral;
                break;
            }
        }
    }

    let local_name = cube_perip
        .properties()
        .await
        .unwrap()
        .unwrap()
        .local_name
        .unwrap();

    print!(
        "Found cube: {} ({}). Connect? [Y/n] ",
        local_name.trim(),
        cube_perip.address()
    );
    io::stdout().flush().unwrap();

    let answer = io::stdin().lines().next().unwrap().unwrap();
    let answer = answer.trim();
    if answer != "y" && answer != "" {
        println!("Disconnecting...");
        let _ = cube_perip.disconnect().await;
        println!("Disconnected. Bye.");
        std::process::exit(0);
    }

    println!("Connecting...");
    cube_perip.connect().await.unwrap();
    println!("Connected.");
    cube_perip.discover_services().await.unwrap();

    let fff6_chr = cube_perip
        .characteristics()
        .into_iter()
        .find(|c| c.uuid == uuid_from_u16(0xfff6))
        .unwrap();

    let cube = Cube::new(cube_perip, fff6_chr);
    crate::cube::run_protocol(cube).await;
}

fn main() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            async_main().await;
        });
}
