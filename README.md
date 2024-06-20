The [QiYi Smart Cube](https://speedcubeshop.com/products/qiyi-ai-3x3-bluetooth-smart-cube-speed-version) is the cheapest of the newly invented genre of bluetooth-enabled "smart" Rubik's cubes. Unfortunately QiYi has refused to publish the protocol used by the cube, and until now there hasn't been much progress in reverse engineering it. This document provides a best effort to reverse engineer and document the protocol, but it is not a complete specification. If you discover anything new please send a pull request!

This document assumes you are somewhat familiar with Bluetooth Low Energy/GATT. If you are new to it, I recommend reading [this introductory article](https://learn.adafruit.com/introduction-to-bluetooth-low-energy/gatt).

# Reference
This repo contains a [reference implementation](reference_app) of the protocol.

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

For cube->app messages, there is a 16 bit little-endian "opcode" after the length. The opcode specifies the type of message. **app->cube messages do not have an opcode.** (TODO: Is there a way to know which kind of message they are without just guessing from the length?)

These are the kinds messages:
- [*App Hello*](#app-hello)
- [*Cube Hello*](#cube-hello)
- [*ACK*](#message-acknowledgement)
- [*State Change*](#state-change-notification)
- [*Sync State*](#sync-state)
- [*Sync Confirmation*](#sync-confirmation)

## Checksum

The last 2 bytes of each message (before the zero padding) are a checksum of the message (minus any zero padding) using the [CRC-16-MODBUS](https://www.modbustools.com/modbus_crc16.html) algorithm. The checksum is in little-endian. Example:
<code>fe 09 02 00 02 45 2c <b>ef 1b</b> 00 00 00 00 00 00 00</code>
Here, the bolded part (`ef 1b`) is the little-endian checksum of `fe 09 02 00 02 45 2c`. So for this example the checksum is `0x1bef`.

## App Hello
|Command|Direction|
|-|-|
|*App Hello*|app->cube|

Immediately after connecting to the cube, you need to write an "*App Hello*" message to the `fff6` characteristic. The *App Hello* must be the first thing you send to the cube. The cube won't reply to **anything** you send unless you've already performed the *App Hello*.

The MAC address field needs to be reversed; the following example is for a cube with the address `cc a3 00 00 25 13`:
```
L = length
A = cube MAC address (but the bytes are backwards!)
C = checksum

   L                ??                         A           C
   /\ /------------------------------\ /---------------\ /---\
fe 15 00 6b 01 00 00 22 06 00 02 08 00 13 25 00 00 a3 cc XX XX
```
|Bytes (start index, length)|Type|Description|
|-|-|-|
|1, 1|u8|Length (always 21 for *App Hello*)|
|2, 11|?|Unknown, but doesn't seem to matter what data is in here; you can just fill it with zeros.|
|13, 6|-|The MAC address of the cube, backwards|
|19, 2|u16_le|Checksum|

## Message Acknowledgement
|Command|Direction|
|-|-|
|*ACK*|app->cube|

Upon receiving most cube->app messages, you have to send an *ACK* message back to the cube. This is the ACK format:
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

*Not all* types of cube->app messages need to be ACKed - see the "Needs ACK?" section in the respective command's descriptions.

TODO: seems like state change notifs only need to get ACKed sometimes? Maybe there's a field in the state change command to request an ACK?

## Cube Hello
|Command|Direction|Needs ACK?|
|-|-|-|
|*Cube Hello*|cube->app|yes|

The "*Cube Hello*" message is sent by the cube immediately after it receives the [*App Hello*](#app-hello).

```
L = length (38)
O = opcode (0x2)
S = initial cube state
C = checksum

   L    O      ?                                         S                                              ?     C
   /\ /---\ /------\ /------------------------------------------------------------------------------\ /---\ /---\
fe 26 02 00 0e 2d aa 33 33 33 33 13 11 11 11 11 44 44 44 44 24 22 22 22 22 00 00 00 00 50 55 55 55 55 00 64 XX XX
```
|Bytes (start index, length)|Type|Description|
|-|-|-|
|1, 1|u8|Length (always 38 for *Cube Hello*)|
|2, 2|u16_le|Opcode (0x2 for *Cube Hello*)|
|4, 3|?|Unknown|
|7, 27|[CubeState](#cube-state-format)|Initial cube state|
|34, 2|?|Unknown|
|36, 2|u16_le|Checksum|

## State Change Notification
|Command|Direction|Needs ACK?|
|-|-|-|
|*State Change*|cube->app|yes|

TODO: at some point the app can stop sending ACKs until the cube is solved???

```
L = length (94)
O = opcode (0x3)
T = current turn
S = cube state
P = previous turns
C = checksum

   L    O      T                                         S                                              ?                                                                      P                                                                                                      C
   /\ /---\ /------\ /------------------------------------------------------------------------------\ /---\ /---------------------------------------------------------------------------------------------------------------------------------------------------------------------\ /---\
fe 5e 03 00 06 98 e5 33 33 33 33 13 11 11 11 11 44 44 44 44 24 22 22 22 22 00 00 00 00 50 55 55 55 55 08 64 ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff 00 03 00 c6 01 00 03 04 ee 02 00 06 94 ab 07 01 XX XX
```

|Bytes (start index, length)|Type|Description|
|-|-|-|
|1, 1|u8|Length (always 94 for *State Change*)|
|2, 2|u16_le|Opcode (0x3 for *State Change*)|
|4, 3|?|Unknown (Current turn??)|
|7, 27|[CubeState](#cube-state-format)|Cube state|
|34, 2|?|Unknown|
|36, 56|?|Unknown (Previous turns??)|
|92, 2|u16_le|Checksum|

## Sync State
|Command|Direction|
|-|-|
|*Sync State*|app->cube|

If the physical state of the cube becomes out of sync with what the cube thinks it is, you can send the *Sync State* command to tell the cube to reset its remembered state to the one you provide in the command. When the cube recieves a *Sync State* command it will reply with a [*Sync Confirmation*](#sync-confirmation) command.

```
L = length (38)
S = new state to set the cube into
C = checksum

   L        ?                                                S                                          ?     C
   /\ /------------\ /------------------------------------------------------------------------------\ /---\ /---\
fe 26 04 17 88 8b 31 33 33 33 33 13 11 11 11 11 44 44 44 44 24 22 22 22 22 00 00 00 00 50 55 55 55 55 00 00 XX XX
```

|Bytes (start index, length)|Type|Description|
|-|-|-|
|1, 1|u8|Length (always 38 for *Sync State*)|
|2, 5|?|Unknown|
|7, 27|[CubeState](#cube-state-format)|The state to set the cube to|
|34, 2|?|Unknown|
|36, 2|u16_le|Checksum|

## Sync Confirmation
|Command|Direction|Needs ACK?|
|-|-|-|
|*Sync Confirmation*|cube->app|no|

Sent in response to a [*Sync State*](#sync-state) command.

```
L = length (38)
O = opcode (0x4)
S = cube's current state
C = checksum

   L    O       ?                                             S                                         ?     C
   /\ /---\ /------\ /------------------------------------------------------------------------------\ /---\ /---\
fe 26 04 00 00 df cc 33 33 33 33 13 11 11 11 11 44 44 44 44 24 22 22 22 22 00 00 00 00 50 55 55 55 55 00 64 XX XX
```

|Bytes (start index, length)|Type|Description|
|-|-|-|
|1, 1|u8|Length (always 38 for *Sync Confirmation*)|
|2, 2|u16_le|Opcode (0x4 for *Sync Confirmation*)|
|4, 3|?|Unknown|
|7, 27|[CubeState](#cube-state-format)|State the cube now thinks it's in|
|34, 2|?|Unknown|
|36, 2|u16_le|Checksum|

# Cube State Format
Cube states are stored as a 54-item-long array of 4-bit numbers, where each 4-bit number represents the color of a facelet (see table below). The index of the item in the array tells you where on the cube the facelet is.

|Number|Color|
|-|-|
|0|orange|
|1|red|
|2|yellow|
|3|white|
|4|green|
|5|blue|

TODO: document the order/layout of each color as used in the [reference app](reference_app/src/cubestate.rs)

A solved cube looks like this:
```
33 33 33 33 13 11 11 11 11 44 44 44 44 24 22 22 22 22 00 00 00 00 50 55 55 55 55
```
