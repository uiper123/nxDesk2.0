# Data Flow Diagrams — TTGTiSO-Desk

This document outlines the detailed sequence and structure of data routing within TTGTiSO-Desk.

## 1. Video Streaming Pipeline (Server -> Client)

The video streaming data flow processes virtual framebuffer changes, encodes them, frames them in TTMP, and pushes them down the SSH channel.

```mermaid
graph TD
    X11[":10 Virtual Framebuffer"] -->|MIT-SHM / XDamage| Capturer["x11rb Capturer"]
    Capturer -->|Raw BGRx Frame Buffers| GstEncoder["GStreamer (VAAPI / OpenH264)"]
    GstEncoder -->|H.264 NAL Chunks| Framing["protocol Crate (Channel 0x01)"]
    Framing -->|TTMP Frames| Transport["transport Crate (SSH Tunnel)"]
    
    %% Network Boundary
    Transport -->|SSH Decryption| ClientTransport["Tauri Transport (Client)"]
    ClientTransport -->|Unframed H.264 Chunks| VideoDecoder["Client Video Decoder (WebCodecs / Gst)"]
    VideoDecoder -->|Raw Video Frame| UICanvas["React UI HTML5 Canvas"]
```

---

## 2. Input Injection Pipeline (Client -> Server)

Input actions (clicks, drags, keyboard hotkeys) are serialized in the client UI and sent to the server for emulation.

```mermaid
graph TD
    UIEvent["User Mouse/Keyboard Action"] -->|React Event Handler| UIBridge["Tauri IPC Command"]
    UIBridge -->|Input Event DTO| ClientFraming["protocol Crate (Channel 0x02)"]
    ClientFraming -->|TTMP Frame| ClientTransport["Tauri Transport (SSH Tunnel)"]
    
    %% Network Boundary
    ClientTransport -->|SSH Channel Read| ServerTransport["Agent Transport"]
    ServerTransport -->|Unpacked Struct| InputInj["input-injector Crate"]
    InputInj -->|XTestFakeKeyEvent / XTestFakeMotionEvent| X11[":10 X11 input queue"]
```

---

## 3. Clipboard Synchronization (Bi-directional)

Clipboard synchronization maintains consistency between client and host clipboard spaces.

```mermaid
sequenceDiagram
    autonumber
    participant ClientOS as Client OS Clipboard
    participant Client as Tauri Client App
    participant Agent as Server Agent
    participant HostOS as Server OS Clipboard

    %% Client to Server Copy
    ClientOS->>Client: Text Copied / Focus Gained
    Client->>Client: Read clipboard string
    Client->>Agent: TTMP (Channel 0x03) - Clipboard Text Payload
    Agent->>HostOS: Write string to display's CLIPBOARD selection

    %% Server to Client Copy
    HostOS->>Agent: CLIPBOARD selection notify event
    Agent->>Agent: Read X11 clipboard selection
    Agent->>Client: TTMP (Channel 0x03) - Clipboard Text Payload
    Client->>ClientOS: Set system clipboard text
```

---

## 4. File Transfer (Upload: Client -> Server)

Files are split into fixed size chunks (e.g. 64KB) to avoid memory overload and sent over Channel 0x04.

```mermaid
sequenceDiagram
    autonumber
    participant ClientUI as React File Transfer UI
    participant ClientCore as Tauri Client Core
    participant Agent as Server Agent
    participant Disk as Target Host File System
    participant Audit as Audit Storage

    ClientUI->>ClientCore: Request Upload (Path, Size, Destination)
    ClientCore->>Agent: FileMetadataRequest (Filename, Total Size)
    Agent->>Agent: Validate path, check space
    Agent-->>ClientCore: FileMetadataResponse (Approved, ResumeOffset)
    
    loop For each 64KB chunk
        ClientCore->>ClientCore: Read chunk from disk
        ClientCore->>Agent: FileChunk (Offset, Data, Checksum)
        Agent->>Disk: Write chunk to temp file
        Agent-->>ClientCore: FileChunkAck (Offset)
    end
    
    ClientCore->>Agent: FileTransferComplete
    Agent->>Disk: Move temp file to destination, verify checksum
    Agent->>Audit: Audit: File upload complete (success=true)
    Agent-->>ClientCore: TransferResult (Success)
```
