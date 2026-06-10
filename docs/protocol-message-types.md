# Protocol Message Types — TTGTiSO Multiplex Protocol (TTMP)

This document defines the application-level message structures used on each multiplexed channel of TTMP.

## 1. Control Channel (0x00)

The control channel handles capability negotiation, session initialization, lifecycle states, and error propagation. All payloads on this channel are JSON-serialized.

### 1.1. Client Messages

#### `ClientHello`
Sent by the client immediately after connection to start capability negotiation.
```json
{
  "type": "ClientHello",
  "client_version": "1.0.0",
  "supported_capabilities": ["h264_vaapi", "h264_software", "clipboard_text", "file_transfer_v1"]
}
```

#### `StartSessionRequest`
Sent by the client to request launch of a new isolated graphical session.
```json
{
  "type": "StartSessionRequest",
  "width": 1920,
  "height": 1080,
  "fps": 30
}
```

#### `StopSessionRequest`
Sent to explicitly request destruction of the current session.
```json
{
  "type": "StopSessionRequest",
  "session_id": "vladimir-10"
}
```

### 1.2. Server Messages

#### `ServerHello`
Response to `ClientHello` confirming negotiated capabilities.
```json
{
  "type": "ServerHello",
  "server_version": "1.0.0",
  "negotiated_capabilities": ["h264_software", "clipboard_text", "file_transfer_v1"]
}
```

#### `SessionStarted`
Sent when the virtual session and GStreamer pipeline have successfully launched.
```json
{
  "type": "SessionStarted",
  "session_id": "vladimir-10",
  "display": ":10"
}
```

#### `ErrorMessage`
Indicates a protocol or runtime failure.
```json
{
  "type": "ErrorMessage",
  "code": 401,
  "message": "Authentication failed or session limit exceeded"
}
```

---

## 2. Video Channel (0x01)

The video channel streams raw H.264 video chunks. Payloads use a binary header followed by NAL units.

```text
+-------------------+----------------+--------------------+
| Timestamp (8B u64)| FrameType (1B) | Raw H.264 Payload  |
+-------------------+----------------+--------------------+
```

- **FrameType values:**
  - `0x01`: Key Frame (I-frame)
  - `0x02`: Delta Frame (P-frame)

---

## 3. Input Channel (0x02)

Binary-serialized structures for low overhead.

### 3.1. Mouse Move / Button
- **Format:**
  - `Event Type` (1B): `0x01` (Move), `0x02` (Press), `0x03` (Release), `0x04` (Scroll)
  - `Button` (1B): `0x01` (Left), `0x02` (Right), `0x03` (Middle)
  - `X` (2B `u16` Big-Endian)
  - `Y` (2B `u16` Big-Endian)
  - `Scroll Delta` (2B `i16` Big-Endian)

### 3.2. Key State
- **Format:**
  - `Event Type` (1B): `0x05` (Press), `0x06` (Release)
  - `Keysym` (4B `u32` Big-Endian): X11 keysym index.

---

## 4. Clipboard Channel (0x03)

UTF-8 encoded string of the clipboard content.

---

## 5. File Transfer Channel (0x04)

Uses a hybrid JSON metadata + binary chunk approach.

#### `FileMetadata` (JSON Payload)
```json
{
  "transfer_id": "tx-9872",
  "file_name": "report.pdf",
  "file_size": 2561024,
  "is_upload": true
}
```

#### `FileChunk` (Binary Payload)
```text
+-------------------+--------------------+--------------------+
| TransferID (8B)   | Offset (8B u64)    | Chunk Data (Bytes) |
+-------------------+--------------------+--------------------+
```

---

## 6. Audit Channel (0x05)

Real-time telemetry and auditing records generated on the server (JSON-serialized).
```json
{
  "timestamp": 1718012920,
  "user": "operator_pavel",
  "event_type": "FILE_TRANSFER",
  "details": "Initiated download of report.pdf (2.4MB)"
}
```

---

## 7. Heartbeat Channel (0x06)

No payload. A simple keep-alive frame sent between client and server to verify link status.
- `Flags`: `0x01` for Ping, `0x02` for Pong.
