import React, { useState, useEffect, useRef } from "react";
import styles from "./ActiveSession.module.css";
import { apiService, AppInfo } from "../../services/api";
// @ts-ignore
import RFB from "@novnc/novnc";

interface ActiveSessionProps {
    host: string;
    port: number;
    username: string;
    displayId?: number;
    onDisconnect: () => void;
}

export const ActiveSession: React.FC<ActiveSessionProps> = ({ host, port: _port, username, displayId, onDisconnect }) => {
    const [fps, setFps] = useState(30);
    const [bitrate, setBitrate] = useState(2400);
    const [latency, setLatency] = useState(12);
    const [scale, setScale] = useState<number | "auto">("auto");
    const [isFullscreen, setIsFullscreen] = useState(false);
    const [clipboardText, setClipboardText] = useState("Copied from host...");
    const [showFileTransfer, setShowFileTransfer] = useState(false);
    const [showAppManager, setShowAppManager] = useState(false);
    const [uploadedFiles, setUploadedFiles] = useState<{name: string, status: string}[]>([]);
    const [apps, setApps] = useState<AppInfo[]>([]);
    const [loadingApps, setLoadingApps] = useState(false);
    const [searchQuery, setSearchQuery] = useState("");

    useEffect(() => {
        if (showAppManager) {
            setLoadingApps(true);
            apiService.getApplications(host)
                .then(res => {
                    setApps(res.applications || []);
                })
                .catch(err => {
                    console.error("Failed to load applications:", err);
                })
                .finally(() => {
                    setLoadingApps(false);
                });
        }
    }, [showAppManager, host]);

    const handleLaunchApp = async (command: string) => {
        try {
            const activeSessions = await apiService.getActiveSessions();
            const matching = activeSessions.find(
                s => s.host_ip === host && s.username === username
            );
            
            let sessionId = matching?.id;
            if (!sessionId) {
                const sysDisplay = activeSessions.find(
                    s => s.host_ip === host && s.id.startsWith("system-display-")
                );
                sessionId = sysDisplay?.id || "system-display-10";
            }

            const res = await apiService.launchApplication(sessionId, command);
            if (res.success) {
                alert(`Приложение "${command}" успешно запущено на удаленном хосте!`);
            } else {
                alert(`Ошибка при запуске: ${res.message}`);
            }
        } catch (e: any) {
            console.error("Launch app error:", e);
            alert(`Не удалось запустить приложение: ${e.message || e}`);
        }
    };

    const [connectionStatus, setConnectionStatus] = useState<"connecting" | "connected" | "disconnected" | "error">("connecting");
    const [errorMessage, setErrorMessage] = useState<string | null>(null);
    const [isCaptured, setIsCaptured] = useState(false);
    const [retryCount, setRetryCount] = useState(0);

    const containerRef = useRef<HTMLDivElement>(null);
    const rfbContainerRef = useRef<HTMLDivElement>(null);
    const rfbRef = useRef<any>(null);

    // Initialize noVNC connection
    useEffect(() => {
        if (!rfbContainerRef.current) return;

        // Clean up any existing connection first
        if (rfbRef.current) {
            try {
                rfbRef.current.disconnect();
            } catch (e) {
                console.error(e);
            }
            rfbRef.current = null;
        }

        const wsUrl = `ws://127.0.0.1:3001/api/ws/vnc?host=${host}&display=${displayId ?? 0}`;
        console.log("Connecting noVNC to:", wsUrl);

        setConnectionStatus("connecting");
        setErrorMessage(null);

        const options = {
            shared: true,
            wsProtocols: ["binary"],
        };

        let rfbInstance: any;
        try {
            rfbInstance = new RFB(rfbContainerRef.current, wsUrl, options);
            rfbRef.current = rfbInstance;

            rfbInstance.focusOnClick = true;

            rfbInstance.addEventListener("connect", () => {
                console.log("noVNC connected successfully");
                setConnectionStatus("connected");
            });

            rfbInstance.addEventListener("disconnect", (e: any) => {
                console.log("noVNC disconnected:", e);
                setConnectionStatus("disconnected");
                if (e.detail && e.detail.clean === false) {
                    setConnectionStatus("error");
                    setErrorMessage("Не удалось подключиться к сессии. Убедитесь, что дисплей существует и запущен.");
                }
            });

            rfbInstance.addEventListener("clipboard", (e: any) => {
                if (e.detail && e.detail.text) {
                    setClipboardText(e.detail.text);
                }
            });

            rfbInstance.scaleViewport = scale === "auto";
            rfbInstance.resizeSession = false;

        } catch (err: any) {
            console.error("Error creating RFB instance:", err);
            setConnectionStatus("error");
            setErrorMessage(`Ошибка подключения: ${err.message || err}`);
        }

        return () => {
            if (rfbRef.current) {
                try {
                    rfbRef.current.disconnect();
                } catch (e) {
                    console.error("Error disconnecting VNC:", e);
                }
                rfbRef.current = null;
            }
        };
    }, [host, displayId, retryCount]);

    // Handle container click (focus input capture)
    const handleContainerClick = () => {
        setIsCaptured(true);
        if (rfbRef.current) {
            rfbRef.current.focus();
        }
    };

    // Keyboard capture for Right Ctrl to exit focus
    useEffect(() => {
        const handleKeyDown = (e: KeyboardEvent) => {
            // Right Ctrl: Release capture
            if (e.key === "Control" && e.location === 2) {
                if (rfbRef.current) {
                    rfbRef.current.blur();
                }
                setIsCaptured(false);
            }
        };

        window.addEventListener("keydown", handleKeyDown);
        return () => {
            window.removeEventListener("keydown", handleKeyDown);
        };
    }, []);

    // Telemetry simulation
    useEffect(() => {
        const interval = setInterval(() => {
            if (connectionStatus === "connected") {
                setFps(30 + Math.floor(Math.random() * 2) - 1);
                setBitrate(2000 + Math.floor(Math.random() * 800));
                setLatency(10 + Math.floor(Math.random() * 5));
            } else {
                setFps(0);
                setBitrate(0);
                setLatency(0);
            }
        }, 1500);
        return () => clearInterval(interval);
    }, [connectionStatus]);

    const handleRetry = () => {
        setRetryCount(prev => prev + 1);
    };

    const handleCopyClipboard = () => {
        navigator.clipboard.writeText(clipboardText);
        if (rfbRef.current) {
            rfbRef.current.clipboardPasteFrom(clipboardText);
        }
        alert("Буфер обмена синхронизирован!");
    };

    return (
        <div className={`${styles.container} ${isFullscreen ? styles.fullscreen : ""}`}>
            {/* Top Toolbar */}
            <div className={styles.toolbar}>
                <div className={styles.meta}>
                    <span className={styles.hostBadge}>{username}@{host}</span>
                    <span className={styles.displayBadge}>Display :{displayId ?? "?"} (Astra SE)</span>
                </div>

                <div className={styles.telemetry}>
                    <div className={styles.metric}>
                        <label>FPS</label>
                        <span>{fps}</span>
                    </div>
                    <div className={styles.metric}>
                        <label>Bitrate</label>
                        <span>{(bitrate / 1000).toFixed(1)} Mbps</span>
                    </div>
                    <div className={styles.metric}>
                        <label>Latency</label>
                        <span className={latency > 15 ? styles.warn : ""}>{latency} ms</span>
                    </div>
                    <div className={styles.qualityIndicator}>
                        <div className={styles.bar}></div>
                        <div className={styles.bar}></div>
                        <div className={styles.bar}></div>
                        <span className={styles.qualText}>Excellent</span>
                    </div>
                </div>

                <div className={styles.controls}>
                    <button 
                        className={styles.toolButton} 
                        onClick={() => {
                            setShowFileTransfer(prev => !prev);
                            setShowAppManager(false);
                        }}
                    >
                        📁 File Transfer
                    </button>
                    <button 
                        className={styles.toolButton} 
                        onClick={() => {
                            setShowAppManager(prev => !prev);
                            setShowFileTransfer(false);
                        }}
                    >
                        🚀 Менеджер приложений
                    </button>
                    <button 
                        className={styles.toolButton} 
                        onClick={() => {
                            setScale(s => {
                                const next = s === "auto" ? 100 : s === 100 ? 75 : s === 75 ? 50 : "auto";
                                if (rfbRef.current) rfbRef.current.scaleViewport = next === "auto";
                                return next;
                            });
                        }}
                    >
                        🔍 Scale: {scale === "auto" ? "Auto" : `${scale}%`}
                    </button>
                    <button 
                        className={styles.toolButton} 
                        onClick={() => {
                            if (!document.fullscreenElement) {
                                containerRef.current?.requestFullscreen().catch(err => {
                                    alert(`Error attempting to enable fullscreen: ${err.message}`);
                                });
                            } else {
                                document.exitFullscreen();
                            }
                            setIsFullscreen(!document.fullscreenElement);
                        }}
                    >
                        📺 {isFullscreen ? "Exit Fullscreen" : "Fullscreen"}
                    </button>
                    <button className={styles.disconnectButton} onClick={onDisconnect}>
                        Disconnect
                    </button>
                </div>
            </div>

            {/* Main Content Area */}
            <div className={styles.content}>
                {isCaptured && (
                    <div className={styles.captureHint}>
                        Захват ввода активен. Нажмите <b>Правый Ctrl</b> для выхода.
                    </div>
                )}

                {connectionStatus === "connecting" && (
                    <div className={styles.connectingText}>
                        <div className={styles.spinner}></div>
                        <span>Подключение к {username}@{host} (Дисплей :{displayId ?? 0})...</span>
                    </div>
                )}

                {connectionStatus === "error" && (
                    <div className={styles.errorText}>
                        <div className={styles.errorIcon}>⚠️</div>
                        <div className={styles.errorMessage}>{errorMessage}</div>
                        <button className={styles.retryButton} onClick={handleRetry}>
                            Повторить попытку
                        </button>
                    </div>
                )}

                {/* Virtual Desktop Display Canvas */}
                <div 
                    ref={containerRef}
                    className={styles.canvasContainer}
                    onClick={handleContainerClick}
                    style={{ 
                        position: "relative",
                        cursor: isCaptured ? "none" : "pointer",
                        display: (connectionStatus === "connected" || connectionStatus === "disconnected") ? "flex" : "none",
                        overflow: scale === "auto" ? "hidden" : "auto",
                        alignItems: "center",
                        justifyContent: "center",
                        backgroundColor: "#000",
                        width: "100%",
                        height: "100%"
                    }}
                >
                    <div 
                        ref={rfbContainerRef}
                        style={{
                            transform: scale !== "auto" ? `scale(${scale / 100})` : "none",
                            transformOrigin: "top left",
                            width: "100%",
                            height: "100%",
                            display: "flex",
                            alignItems: "center",
                            justifyContent: "center"
                        }}
                    />
                </div>

                {/* Slide-out File Transfer Sidebar */}
                {showFileTransfer && (
                    <div className={styles.sidebar}>
                        <div className={styles.sidebarHeader}>
                            <h3>Файловый менеджер</h3>
                            <button onClick={() => setShowFileTransfer(false)}>✕</button>
                        </div>
                        <div className={styles.sidebarContent}>
                            <div 
                                className={styles.uploadBox}
                                onDragOver={(e) => { e.preventDefault(); e.stopPropagation(); }}
                                onDrop={async (e) => {
                                    e.preventDefault();
                                    e.stopPropagation();
                                    const files = Array.from(e.dataTransfer.files);
                                    for (const file of files) {
                                        setUploadedFiles(prev => [...prev, { name: file.name, status: "Загрузка..." }]);
                                        try {
                                            const res = await fetch(`http://127.0.0.1:3001/api/upload/${encodeURIComponent(file.name)}`, {
                                                method: "POST",
                                                body: file
                                            });
                                            if (res.ok) {
                                                setUploadedFiles(prev => prev.map(f => f.name === file.name ? { name: file.name, status: "Загружено на Рабочий Стол" } : f));
                                            } else {
                                                setUploadedFiles(prev => prev.map(f => f.name === file.name ? { name: file.name, status: "Ошибка" } : f));
                                            }
                                        } catch (err) {
                                            setUploadedFiles(prev => prev.map(f => f.name === file.name ? { name: file.name, status: "Ошибка сети" } : f));
                                        }
                                    }
                                }}
                            >
                                <p>Перетащите файлы сюда</p>
                                <span>(Файлы будут сохранены на Рабочий Стол агента)</span>
                            </div>
                            <div className={styles.fileList}>
                                {uploadedFiles.map((file, idx) => (
                                    <div key={idx} className={styles.fileItem}>
                                        <span>{file.name}</span>
                                        <span className={styles.statusBadge}>{file.status}</span>
                                    </div>
                                ))}
                                {uploadedFiles.length === 0 && (
                                    <div className={styles.noAppsText}>Нет загруженных файлов</div>
                                )}
                            </div>
                        </div>
                    </div>
                )}

                {/* Slide-out Application Manager Sidebar */}
                {showAppManager && (
                    <div className={styles.sidebar}>
                        <div className={styles.sidebarHeader}>
                            <h3>Менеджер приложений</h3>
                            <div style={{ display: "flex", gap: "8px" }}>
                                <button 
                                    onClick={() => handleLaunchApp("xdotool getactivewindow windowkill || wmctrl -c :ACTIVE:")}
                                    style={{ 
                                        backgroundColor: "rgba(255, 50, 50, 0.2)", 
                                        color: "#ff5555", 
                                        border: "1px solid #ff5555", 
                                        borderRadius: "4px", 
                                        padding: "2px 8px", 
                                        cursor: "pointer",
                                        fontSize: "12px"
                                    }}
                                    title="Закрыть активное окно на удаленном рабочем столе"
                                >
                                    Закрыть окно
                                </button>
                                <button onClick={() => setShowAppManager(false)}>✕</button>
                            </div>
                        </div>
                        <div className={styles.sidebarContent}>
                            <input
                                type="text"
                                placeholder="Поиск приложений..."
                                value={searchQuery}
                                onChange={(e) => setSearchQuery(e.target.value)}
                                className={styles.appSearchInput}
                            />
                            {loadingApps ? (
                                <div className={styles.appLoading}>Получение списка ПО...</div>
                            ) : (
                                <div className={styles.appList}>
                                    {apps
                                        .filter(app => app.name.toLowerCase().includes(searchQuery.toLowerCase()))
                                        .map((app, idx) => (
                                            <div 
                                                key={idx} 
                                                className={styles.appItem}
                                                onClick={() => handleLaunchApp(app.exec)}
                                            >
                                                <div className={styles.appIcon}>🚀</div>
                                                <div className={styles.appInfoText}>
                                                    <div className={styles.appName}>{app.name}</div>
                                                    <div className={styles.appExec}>{app.exec}</div>
                                                </div>
                                            </div>
                                        ))
                                    }
                                    {apps.length === 0 && (
                                        <div className={styles.noAppsText}>Приложения не найдены</div>
                                    )}
                                </div>
                            )}
                        </div>
                    </div>
                )}
            </div>

            {/* Bottom Clipboard Sync Bar */}
            <div className={styles.clipboardBar}>
                <span className={styles.clipLabel}>Clipboard Status:</span>
                <input 
                    type="text" 
                    value={clipboardText} 
                    onChange={(e) => setClipboardText(e.target.value)} 
                    className={styles.clipInput}
                />
                <button className={styles.syncButton} onClick={handleCopyClipboard}>
                    Sync to Local Clipboard
                </button>
            </div>
        </div>
    );
};
