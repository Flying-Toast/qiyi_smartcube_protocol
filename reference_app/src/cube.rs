use crate::crc::crc16;
use crate::cubestate;
use crate::messages::{self, C2aBody};
use aes::{
    cipher::{BlockDecrypt, BlockEncrypt, KeyInit},
    Aes128, Block,
};
use btleplug::api::{Characteristic, Peripheral as _, WriteType};
use btleplug::platform::Peripheral;
use futures_util::StreamExt;

pub struct Cube {
    perip: Peripheral,
    fff6: Characteristic,
    cipher: Aes128,
}

impl Cube {
    pub fn new(perip: Peripheral, fff6: Characteristic) -> Self {
        Self {
            perip,
            fff6,
            cipher: Aes128::new(
                &[
                    87, 177, 249, 171, 205, 90, 232, 167, 156, 185, 140, 231, 87, 140, 81, 8,
                ]
                .into(),
            ),
        }
    }

    /// Given the bytes of an app->cube command:
    /// - prefixes with `0xfe` and the length;
    /// - computes the checksum and appends it to the end;
    /// - adds zero-padding;
    /// - encrypts the message;
    /// - writes it to the fff6 characteristic
    async fn write_cmd_inner_bytes(&mut self, bytes: &[u8]) {
        // +2 for checksum, +2 for fe/length prefix
        let cmdlen = bytes.len() + 2 + 2;
        let npad = if cmdlen % 16 == 0 {
            0
        } else {
            16 - (cmdlen % 16)
        };
        let total_len = npad + cmdlen;
        assert!(total_len % 16 == 0);

        let mut bytes = {
            let mut v = Vec::<u8>::with_capacity(total_len);
            v.push(0xfe);
            v.push(cmdlen.try_into().expect("Packet len > 255"));
            v.extend_from_slice(bytes);
            v.extend_from_slice(&crc16(&v).to_le_bytes());
            v.resize(total_len, 0);
            v
        };

        // encrypt bytes
        for mut block in bytes.chunks_mut(16).map(Block::from_mut_slice) {
            self.cipher.encrypt_block(&mut block);
        }

        self.perip
            .write(&self.fff6, &bytes, WriteType::WithoutResponse)
            .await
            .unwrap();
    }
}

pub async fn run_protocol(mut cube: Cube) {
    cube.perip.subscribe(&cube.fff6).await.unwrap();

    // send App Hello
    cube.write_cmd_inner_bytes(&messages::make_app_hello(cube.perip.address()))
        .await;

    let mut notifs = cube.perip.notifications().await.unwrap();
    let mut last_ms = 0;
    while let Some(n) = notifs.next().await {
        assert!(n.uuid == cube.fff6.uuid);
        let mut bytes = n.value;
        assert!(bytes.len() % 16 == 0);

        for mut block in bytes.chunks_mut(16).map(Block::from_mut_slice) {
            cube.cipher.decrypt_block(&mut block);
        }

        let msg = messages::parse_c2a_message(&bytes).unwrap();

        if let C2aBody::StateChange(sc) = &msg.body() {
            cubestate::render_cube(&sc.state);
            println!(
                "Turn: {} | ts diff {}",
                sc.turn,
                sc.millis_timestamp - last_ms
            );
            last_ms = sc.millis_timestamp;
        }

        if let Some(pkt) = msg.make_ack() {
            cube.write_cmd_inner_bytes(pkt).await;
        }
    }

    println!("Disconnecting...");
    cube.perip.disconnect().await.unwrap();
    println!("Disconnected.");
}
