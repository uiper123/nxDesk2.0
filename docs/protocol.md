# Protocol Specification — TTGTiSO Multiplex Protocol (TTMP)

This document describes the binary framing and multiplexing protocol used by TTGTiSO-Desk to run multiple logical channels over a single SSH connection.

## 1. Frame Structure

Every packet transmitted over the TTMP channel starts with a fixed-size header followed by a variable-length payload.

```text
+-------------------+------------------+----------------+--------------------+---------------+---------------------+
| Magic (4 bytes)   | Version (1 byte) | ChanID (1 byte)| Length (4 bytes)   | Flags (1 byte)| Payload (N bytes)   |
+-------------------+------------------+----------------+--------------------+---------------+---------------------+
| 'T' 'T' 'G' 'T'   | 0x01             | 0x00 - 0x06    | Big-Endian u32     | Bitmask       | Binary payload      |
+-------------------+------------------+----------------+--------------------+---------------+---------------------+
```

### 1.1. Header Fields
- **Magic Bytes (4 bytes):** Always `0x54 0x54 0x47 0x54` (ASCII representation of "TTGT"). Used to verify stream alignment.
- **Protocol Version (1 byte):** Initial version is `0x01`.
- **Channel ID (1 byte):** The destination channel for the frame (see Section 2).
- **Payload Length (4 bytes):** A `u32` in Big-Endian format indicating the size of the payload in bytes.
- **Flags (1 byte):** Bitmask for control flags:
  - `0x01`: End of message (EOF for multi-frame transfers).
  - `0x02`: Compressed payload (e.g. using zstd).
  - `0x04`: Critical/High-Priority (bypass standard queues).

---

## 2. Channel Definitions

TTMP uses 7 logical channels multiplexed over the underlying SSH connection.

| Channel ID | Name | Direction | Payload Type | Description |
| :--- | :--- | :--- | :--- | :--- |
| **0x00** | **Control** | Bi-directional | JSON | Session management, capability negotiation, state reports. |
| **0x01** | **Video** | Server -> Client | H.264 stream | Video frames/fragments with timestamps and encoding info. |
| **0x02** | **Input** | Client -> Server | Binary Struct | Mouse coordinates, clicks, scroll, and key press/release events. |
| **0x03** | **Clipboard** | Bi-directional | UTF-8 Text | Synchronizing text buffer copy-paste operations. |
| **0x04** | **File** | Bi-directional | Binary / JSON | Chunked file transfer, file requests, metadata. |
| **0x05** | **Audit** | Server -> Client | JSON | Security events, logs, connection telemetry. |
| **0x06** | **Heartbeat** | Bi-directional | None | Keep-alive ping/pong frames to monitor connection health. |

---

## 3. Channel Serialization Formats

### 3.1. Control Channel (0x00)
Uses JSON-encoded structures to ease extensibility.
Example Message (Session Start Request):
```json
{
  "msg_type": "StartSession",
  "username": "vladimir",
  "requested_width": 1920,
  "requested_height": 1080,
  "requested_fps": 30
}
```

### 3.2. Video Channel (0x01)
Consists of raw encoded video chunks. The payload begins with a 12-byte metadata block:
- **Timestamp (8 bytes):** Big-Endian `u64` (microseconds).
- **Frame Type (1 byte):** `0x01` (I-Frame), `0x02` (P-Frame).
- **Reserved (3 bytes):** Padding.
- **Video Data (Remaining bytes):** H.264 NAL units.

### 3.3. Input Channel (0x02)
Uses fixed-size binary structs for speed and minimal overhead.
- **Mouse Event Struct (8 bytes):**
  - Event Type (1 byte): `0x01` (Move), `0x02` (Press), `0x03` (Release), `0x04` (Scroll).
  - Button/Key (1 byte): `0x00` (None), `0x01` (Left), `0x02` (Right), `0x03` (Middle).
  - X Position (2 bytes): Big-Endian `u16`.
  - Y Position (2 bytes): Big-Endian `u16`.
  - Scroll Delta (2 bytes): Big-Endian `i16`.
- **Keyboard Event Struct (5 bytes):**
  - Event Type (1 byte): `0x05` (Press), `0x06` (Release).
  - Keycode (4 bytes): Big-Endian `u32` representing Linux/X11 keysym.

---

## 4. Congestion Control & Heartbeats

- **Backpressure:** The server buffers video frames up to a threshold. If the TCP window is full and frames queue up, the server drops P-frames and reduces GStreamer encoding bitrate.
- **Heartbeat Interval:** A ping is sent every 5 seconds. If no pong is received within 15 seconds, the channel is considered dead, prompting the client to attempt a reconnect.
