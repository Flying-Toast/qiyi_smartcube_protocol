The [QiYi Smart Cube](https://speedcubeshop.com/products/qiyi-ai-3x3-bluetooth-smart-cube-speed-version) is the cheapest of the newly invented genre of bluetooth-enabled "smart" Rubik's cubes. Unfortunately QiYi has refused to publish the protocol used by the cube, and until now there hasn't been much progress in reverse engineering it. This document provides a best effort to reverse engineer and document the protocol, but it is not a complete specification. If you discover anything new please send a pull request!

This document assumes you are somewhat familiar with Bluetooth Low Energy/GATT. If you are new to it, I recommend reading [this introductory article](https://learn.adafruit.com/introduction-to-bluetooth-low-energy/gatt).

I've also created a [Wireshark plugin](wireshark_dissector) that is helpful when doing any work with the cube protocol.

# GATT Profile
The protocol is on top of Bluetooth Low Energy. The cube has the following GATT profile:

- Service `1801`
    - Characteristic `2a05`
- Service `fff0`
    - Characteristic `fff4`
    - Characteristic `fff5`
    - Characteristic `fff6`
    - Characteristic `fff7`
- Service `5833ff01-9b8b-5191-6142-22a4536ef123`
    - Characteristic `5833ff02-9b8b-5191-6142-22a4536ef123`
    - Characteristic `5833ff03-9b8b-5191-6142-22a4536ef123`

A lot of these use the standard base uuid, but as far as I can tell they don't follow the standards and might as well be custom uuids. The majority of the protocol just uses WRITEs and NOTIFYs on the `fff6` characteristic.

# Protocol
app->cube messages are sent by performing a WRITE of the data to the `fff6` characteristic.

cube->app messages are received via NOTIFY events on the `fff6` characteristic.

## Encryption
All messages sent to/received from the cube are encrypted using AES128 in ECB mode with the fixed key `57b1f9abcd5ae8a79cb98ce7578c5108` (`[87, 177, 249, 171, 205, 90, 232, 167, 156, 185, 140, 231, 87, 140, 81, 8]`)

Before being encrypted, **messages are padded with trailing zeros** until the total message length is a multiple of 16.

For the rest of this document, all values are given in their **decrypted form** and it is implied that messages being sent will be encrypted and messages received have been decrypted. It is also implied that the [checksum](#checksum) is included in each message.

## Messages
All messages (both app->cube and cube->app) start with the byte `0xfe`. The next byte is the length of the message (excluding padding).

For cube->app messages, there is a 16 bit little-endian "opcode" after the length. The opcode specifies the type of message. **app->cube messages do not have an opcode.** (TODO: ????)

These are the kinds messages:
- [*App Hello*](#app-hello)
- [*Cube Hello*](#cube-hello)
- [*ACK*](#message-acknowledgement)
- [*State Change*](#state-change-notification)

## Checksum

The last 2 bytes of each message (before the zero padding) are a checksum of the message (minus any zero padding) using the [CRC-16-MODBUS](https://www.modbustools.com/modbus_crc16.html) algorithm. The checksum is in little-endian. Example:
<code>fe 09 02 00 02 45 2c <b>ef 1b</b> 00 00 00 00 00 00 00</code>
Here, the bolded part (`ef 1b`) is the little-endian checksum of `fe 09 02 00 02 45 2c`. So for this example the checksum is `0x1bef`.

## App Hello
|Command|Direction|
|-|-|
|*App Hello*|app->cube|

Immediately after connecting to the cube, you need to write an "*App Hello*" message to the `fff6` characteristic. I don't yet know what this message contains, but you can just send this one verbatim and things will still work:
```
L = length
C = checksum

   L                         ??                            C
   /\ /------------------------------------------------\ /---\
fe 15 00 6b 01 00 00 22 06 00 02 08 00 13 25 00 00 a3 cc ce 38
```
|Bytes (start index, length)|Type|Description|
|-|-|-|
|1, 1|u8|Length (always 21 for *App Hello*)|
|2, 17|?|Unknown|
|19, 2|u16_le|Checksum|

## Cube Hello
|Command|Direction|
|-|-|
|*Cube Hello*|cube->app|

The "*Cube Hello*" message is sent by the cube immediately after it receives the [*App Hello*](#app-hello). You need to [*ACK*](#message-acknowledgement) the *Cube Hello* just like any other cube->app message.

```
L = length (38)
O = opcode (0x2)
IS = initial cube state
C = checksum

   L    O      ?                                       IS                                               ?     C
   /\ /---\ /------\ /------------------------------------------------------------------------------\ /---\ /---\
fe 26 02 00 0e 2d aa 33 33 33 33 13 11 11 11 11 44 44 44 44 24 22 22 22 22 00 00 00 00 50 55 55 55 55 00 64 CC CC
```
|Bytes (start index, length)|Type|Description|
|-|-|-|
|1, 1|u8|Length (always 38 for *Cube Hello*)|
|2, 2|u16_le|Opcode (0x2 for *Cube Hello*)|
|4, 3|?|Unknown|
|7, 27|-|Initial cube state|
|34, 2|?|Unknown|
|36, 2|u16_le|Checksum|

## Message Acknowledgement
|Command|Direction|
|-|-|
|*ACK*|app->cube|

Upon receiving cube->app messages, you have to send an *ACK* message back to the cube. This is the ACK format:
```
L = length (always 9 for ACKs)
H = bytes 3-7 of the message being ACKed
C = checksum

   L       H           C
   /\ /------------\ /---\
fe 09 XX XX XX XX XX CC CC
```
That would be an ACK for a message that looks like this:
```
            H
      /------------\
fe zz XX XX XX XX XX zz zz zz zz zz zz ...
```

TODO: seems like you can also ACK 2 (or more?) messages at once by just ACKing the latest message?

## State Change Notification
|Command|Direction|
|-|-|
|*State Change*|cube->app|

```
L = length (94)
O = opcode (0x3)
T = current turn
S = cube state
P = previous turns
C = checksum

   L    O      T                                         S                                              ?                                                                      P                                                                                                      C
   /\ /---\ /------\ /------------------------------------------------------------------------------\ /---\ /---------------------------------------------------------------------------------------------------------------------------------------------------------------------\ /---\
fe 5e 03 00 06 98 e5 33 33 33 33 13 11 11 11 11 44 44 44 44 24 22 22 22 22 00 00 00 00 50 55 55 55 55 08 64 ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff 00 03 00 c6 01 00 03 04 ee 02 00 06 94 ab 07 01 CC CC
```

|Bytes (start index, length)|Type|Description|
|-|-|-|
|1, 1|u8|Length (always 94 for *State Change*)|
|2, 2|u16_le|Opcode (0x3 for *State Change*)|
|4, 3|-|Current turn|
|7, 27|-|Cube state|
|34, 2|?|Unknown|
|36, 56|Previous turns|
|92, 2|u16_le|Checksum|
