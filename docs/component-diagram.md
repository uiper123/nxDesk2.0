# Component Diagrams — TTGTiSO-Desk

This document shows component dependencies and interactions inside the TTGTiSO-Desk repository.

## 1. Monorepo Dependency Tree

The diagram below maps how crates and packages relate to each other. Directed arrows show compilation dependencies (`A --> B` means A depends on B).

```mermaid
graph TD
    subgraph Apps ["Apps (Executable Binaries)"]
        ClientApp["apps/desktop-client (Tauri App)"]
        ServerAgent["apps/server-agent (Systemd Service)"]
        RelayServer["apps/relay-server"]
        AdminCLI["apps/admin-cli"]
    end

    subgraph Crates ["Core Library Crates"]
        Transport["crates/transport"]
        Protocol["crates/protocol"]
        Security["crates/security"]
        SessionMgr["crates/session-manager"]
        VideoPipe["crates/video-pipeline"]
        InputInj["crates/input-injector"]
        Clipboard["crates/clipboard"]
        FileTransfer["crates/file-transfer"]
        Audit["crates/audit"]
        Config["crates/config"]
        OSPAL["crates/os-pal"]
        SharedTypes["crates/shared-types"]
    end

    %% Client App dependencies
    ClientApp --> Transport
    ClientApp --> Config

    %% Server Agent dependencies
    ServerAgent --> Transport
    ServerAgent --> Security
    ServerAgent --> SessionMgr
    ServerAgent --> VideoPipe
    ServerAgent --> InputInj
    ServerAgent --> Clipboard
    ServerAgent --> FileTransfer
    ServerAgent --> Audit
    ServerAgent --> Config

    %% Relay Server dependencies
    RelayServer --> Transport
    RelayServer --> Config

    %% Admin CLI dependencies
    AdminCLI --> Audit
    AdminCLI --> Config

    %% Shared dependencies between library crates
    Transport --> Protocol
    Protocol --> SharedTypes
    Security --> Config
    SessionMgr --> OSPAL
    SessionMgr --> Config
    VideoPipe --> SharedTypes
    InputInj --> SharedTypes
    FileTransfer --> SharedTypes
    Audit --> SharedTypes
```

---

## 2. Dynamic Component Subsystems

### 2.1. Client UI and Backend Bridge (Tauri IPC)
```mermaid
graph LR
    subgraph UI ["TypeScript/React UI"]
        ReactUI["React Frontend View"]
        TauriAPI["Tauri IPC Client API"]
        ReactUI --> TauriAPI
    end

    subgraph Backend ["Tauri Backend (Rust)"]
        TauriCmds["Tauri Commands Handler"]
        TauriAPI -- "JSON IPC (Specta Bindings)" --> TauriCmds
        TauriCmds --> RustCore["Client-Side transport engine"]
    end
```

### 2.2. Session Capture and Compression Loop
```mermaid
graph LR
    subgraph X11 ["X11 Desktop Session"]
        FB["Virtual Framebuffer"]
    end
    
    subgraph Capturer ["video-pipeline Crate"]
        XShm["MIT-SHM Capture (x11rb)"]
        Gst["GStreamer Encoder (H.264)"]
        FB --> XShm
        XShm --> Gst
    end

    subgraph Network ["Network Transport"]
        Gst --> Multiplexer["protocol/transport Crates"]
    end
```
