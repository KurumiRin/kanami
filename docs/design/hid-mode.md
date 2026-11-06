# HID mode (design draft, not implemented)

> Status: **to be implemented**, planned together with the GUI (a switch in the GUI that guides driver installation and authorization).

## The problem

The default mode is based on `CGEventTap` plus synthesized events, and some things are out of its reach:

- Password fields, apps with Secure Input on (such as Terminal's "Secure Keyboard Entry"), and the lock screen: the system does not hand keys to the event tap, so remapping does not work.
- Software that reads hardware input directly, such as games, virtual machines and remote desktops, does not see remapped keys.
- A few apps that query the hardware modifier state directly do not see synthesized modifiers (the flags of mouse events are already corrected by the default mode).

HID mode lets kanami seize the physical keyboard exclusively and read raw HID input. The engine processes it, and the result is sent through a **virtual keyboard**, so the output is indistinguishable from a real keyboard.

## Why use Karabiner's driver

A virtual keyboard can only be built as a DriverKit system extension. That needs an entitlement approved by Apple (`com.apple.developer.driverkit.family.hid.virtual.device`) and a paid developer account, and the system extension must live in an app from the same signing team. An open-source project has neither, so kanami reuses pqrs.org's [Karabiner-DriverKit-VirtualHIDDevice](https://github.com/pqrs-org/Karabiner-DriverKit-VirtualHIDDevice) (kanata does the same on macOS).

- License: the repository LICENSE is the Unlicense and the source file headers say Boost Software License 1.0. Both allow kanami to download and install the official signed installer automatically.
- kanami does not bundle the driver itself (the signing team differs, and the system does not allow it). Instead it provides `kanami hid install`, which downloads a pinned version of the official `.pkg`, verifies the hash, installs and activates it. The user only has to click "Allow" once in "System Settings → Privacy & Security".

## Requirements

- The driver accepts commands only from root processes, and its own daemon must also run as root:
  `/Library/Application Support/org.pqrs/Karabiner-DriverKit-VirtualHIDDevice/Applications/Karabiner-VirtualHIDDevice-Daemon.app/Contents/MacOS/Karabiner-VirtualHIDDevice-Daemon`
  (upstream recommends keeping it resident with launchd and `ProcessType=Interactive`).
- In HID mode, kanami runs as root (a LaunchDaemon) and seizes the physical keyboard exclusively with `IOHIDManager` (`kIOHIDOptionsTypeSeizeDevice`).
- It cannot be used together with Karabiner-Elements (both need exclusive access to the device).
- macOS 13 or newer is supported.

## Protocol (client protocol 7, from the upstream source)

Transport: a Unix domain **stream** socket, connect only, no bind:
`/Library/Application Support/org.pqrs/tmp/rootonly/karabiner_virtual_hid_device_service.sock`

Framing (pqrs `unix_domain_stream`):

```
[u32 length, big-endian, excluding itself][u8 message type][content]
Message types: 0 heartbeat, 1 user_data, 2 health_check, 3 health_check_response, 4 request, 5 response
Content of request / response: [u64 request_id, big-endian, increasing from 1][payload]
Heartbeat frame: 00 00 00 01 00, sent every 3 seconds; no frame for 30 seconds counts as disconnected
```

Payload (virtual HID service, host byte order, i.e. little-endian):

```
[u16 protocol version = 7][u8 request type][raw bytes of the struct]
Request types: 0 keyboard_initialize, 1 keyboard_terminate, 2 keyboard_reset,
               3 pointing_initialize, 4 pointing_terminate, 5 pointing_reset,
               6 post_keyboard_input_report, 7 post_consumer_input_report,
               8 post_apple_vendor_keyboard_input_report, 9 post_apple_vendor_top_case_input_report,
               10 post_generic_desktop_input_report, 11 post_pointing_input_report
```

Server notifications come in pairs of `[u8 type][u8 bool]`: type 0 none, 1 driver_activated, 2 driver_connected, 3 driver_version_mismatched, 4 virtual_hid_keyboard_ready, 5 virtual_hid_pointing_ready. The server also pushes these notifications as requests, and the client answers with an empty response.

Keyboard initialization parameters (24 bytes, three `u64`): `vendor_id` (default 0x16c0), `product_id` (default 0x27db), `country_code` (33 for US).

Keyboard input report (67 bytes, packed):

```
[0]     report_id = 1
[1]     modifier bitmap: left ⌃ 0x01, left ⇧ 0x02, left ⌥ 0x04, left ⌘ 0x08, right ⌃ 0x10, right ⇧ 0x20, right ⌥ 0x40, right ⌘ 0x80
[2]     reserved 0
[3..67] 32 u16 key codes (HID Keyboard/Keypad page 0x07, such as a = 0x04), unused entries filled with 0
```

Startup order: connect → receive driver_connected → keyboard_initialize → wait for virtual_hid_keyboard_ready → start sending reports after about 500 ms.

## Implementation notes

- The backend turns HID input values into a `Key` and a device and hands them to the same engine. The output `KeyEvent` sequence is reduced to "set of keys currently held + modifier bitmap", and one report is sent on every change.
- Auto-repeat is left to the system (the system generates repeats itself while a virtual keyboard key is held), and repeat events from the engine output are ignored.
- Text from `kanami.type` cannot be sent through HID reports and still uses Unicode synthesized events (which do not work in password fields).
- When the process exits or crashes, the kernel releases the seized device automatically. Before a normal exit, an all-empty report is sent so no pressed key remains on the virtual keyboard.

## To be confirmed

- Whether the `unix_domain_stream` version pinned in upstream `vendor/` uses the same framing as the main branch.
- The permission settings of the server socket (`server.hpp` has not been read).
- That `type_safe::strong_typedef` does not change struct sizes (the initialization parameters are computed as 24 bytes, not verified by compilation).
