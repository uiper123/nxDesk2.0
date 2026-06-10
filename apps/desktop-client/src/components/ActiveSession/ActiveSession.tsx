import React, { useState, useEffect, useRef, useMemo } from "react";
import styles from "./ActiveSession.module.css";
import { apiService, AppInfo, API_BASE_URL } from "../../services/api";
import { useToast } from "../Toast";
import { logger } from "../../services/logger";
import {
    buildRemoteDesktopUrls,
    buildUploadUrl,
    classifyConnectionHealth,
    ConnectionMode,
    formatDuration,
    formatHostEndpoint,
    getConnectionModeDetails,
} from "./remoteAccess";
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
    const { showToast } = useToast();
    const [fps, setFps] = useState(30);
    const [bitrate, setBitrate] = useState(2400);
    const [latency, setLatency] = useState(12);
    const [scale, setScale] = useState<number | "auto">("auto");
    const [isFullscreen, setIsFullscreen] = useState(false);
    const [clipboardText, setClipboardText] = useState("Copied from host...");
    const [clipboardSynced, setClipboardSynced] = useState(false);
    const [showFileTransfer, setShowFileTransfer] = useState(false);
    const [showAppManager, setShowAppManager] = useState(false);
    const [uploadedFiles, setUploadedFiles] = useState<{name: string, status: string}[]>([]);
    const [apps, setApps] = useState<AppInfo[]>([]);
    const [loadingApps, setLoadingApps] = useState(false);
    const [searchQuery, setSearchQuery] = useState("");
    const [connectionMode, setConnectionMode] = useState<ConnectionMode>("balanced");
    const [showOverview, setShowOverview] = useState(false);

    const [connectionStatus, setConnectionStatus] = useState<"connecting" | "connected" | "disconnected" | "error">("connecting");
    const [errorMessage, setErrorMessage] = useState<string | null>(null);
    const [isCaptured, setIsCaptured] = useState(false);
    const [retryCount, setRetryCount] = useState(0);
    const [connectedAt, setConnectedAt] = useState<number | null>(null);
    const [sessionSeconds, setSessionSeconds] = useState(0);

    const containerRef = useRef<HTMLDivElement>(null);
    const rfbContainerRef = useRef<HTMLDivElement>(null);
    const rfbRef = useRef<any>(null);
    const scaleRef = useRef<number | "auto">("auto");

    const remoteUrls = useMemo(() => buildRemoteDesktopUrls(API_BASE_URL, host, displayId ?? 0), [host, displayId]);
    const modeDetails = useMemo(() => getConnectionModeDetails(connectionMode), [connectionMode]);
    const connectionHealth = useMemo(
        () =>
            classifyConnectionHealth({
                status: connectionStatus,
                retryCount,
                sessionSeconds,
                clipboardSynced,
            }),
        [clipboardSynced, connectionStatus, retryCount, sessionSeconds],
    );
    const endpointSummary = useMemo(
        () => formatHostEndpoint(host, _port, displayId),
        [host, _port, displayId],
    );

    useEffect(() => {
        scaleRef.current = scale;
        if (rfbRef.current) {
            rfbRef.current.scaleViewport = scale === "auto";
        }
    }, [scale]);

    useEffect(() => {
        if (connectionStatus === "connected") {
            if (!connectedAt) {
                setConnectedAt(Date.now());
            }
            return;
        }

        setConnectedAt(null);
        setSessionSeconds(0);
    }, [connectedAt, connectionStatus]);

    useEffect(() => {
        if (!connectedAt) return;
        const timer = setInterval(() => {
            setSessionSeconds(Math.max(0, Math.floor((Date.now() - connectedAt) / 1000)));
        }, 1000);
        return () => clearInterval(timer);
    }, [connectedAt]);

    useEffect(() => {
        if (showAppManager) {
            setLoadingApps(true);
            apiService.getApplications(host)
                .then(res => {
                    setApps(res.applications || []);
                })
                .catch(err => {
                    logger.error("session", "Failed to load applications", err);
                    showToast("error", "Не удалось загрузить список приложений");
                })
                .finally(() => {
                    setLoadingApps(false);
                });
        }
    }, [showAppManager, host, showToast]);

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
                showToast("success", "Приложение запущено", `«${command}» выполняется на удалённом хосте.`);
            } else {
                showToast("error", "Ошибка при запуске", res.message);
            }
        } catch (e: any) {
            logger.error("session", "Launch app error", e);
            showToast("error", "Не удалось запустить приложение", e.message || String(e));
        }
    };

    // Initialize noVNC connection
    useEffect(() => {
        if (!rfbContainerRef.current) return;

        if (rfbRef.current) {
            try {
                rfbRef.current.disconnect();
            } catch (e) {
                logger.warn("session", "Error during VNC cleanup", e);
            }
            rfbRef.current = null;
        }

        const wsUrl = remoteUrls.wsUrl;
        logger.info("session", `Connecting noVNC to ${wsUrl}`);

        setConnectionStatus("connecting");
        setErrorMessage(null);
        setClipboardSynced(false);

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
                logger.info("session", "noVNC connected successfully");
                setConnectionStatus("connected");
            });

            rfbInstance.addEventListener("disconnect", (e: any) => {
                logger.info("session", "noVNC disconnected", e?.detail);
                setConnectionStatus("disconnected");
                if (e.detail && e.detail.clean === false) {
                    setConnectionStatus("error");
                    setErrorMessage("Не удалось подключиться к сессии. Убедитесь, что дисплей существует и запущен.");
                }
            });

            rfbInstance.addEventListener("clipboard", (e: any) => {
                if (e.detail && e.detail.text) {
                    setClipboardText(e.detail.text);
                    setClipboardSynced(true);
                }
            });

            rfbInstance.scaleViewport = scaleRef.current === "auto";
            rfbInstance.resizeSession = false;
        } catch (err: any) {
            logger.error("session", "Error creating RFB instance", err);
            setConnectionStatus("error");
            setErrorMessage(`Ошибка подключения: ${err.message || err}`);
        }

        return () => {
            if (rfbRef.current) {
                try {
                    rfbRef.current.disconnect();
                } catch (e) {
                    logger.warn("session", "Error disconnecting VNC", e);
                }
                rfbRef.current = null;
            }
        };
    }, [host, displayId, remoteUrls.wsUrl, retryCount]);

    const handleSelectMode = (mode: ConnectionMode) => {
        setConnectionMode(mode);
        const details = getConnectionModeDetails(mode);
        setScale(details.recommendedScale === "fit" ? "auto" : details.recommendedScale);
    };

    const handleContainerClick = () => {
        setIsCaptured(true);
        if (rfbRef.current) {
            rfbRef.current.focus();
        }
    };

    useEffect(() => {
        const handleKeyDown = (e: KeyboardEvent) => {
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
        setConnectionStatus("connecting");
    };

    const handleCopyClipboard = async () => {
        try {
            await navigator.clipboard.writeText(clipboardText);
            if (rfbRef.current) {
                rfbRef.current.clipboardPasteFrom(clipboardText);
            }
            setClipboardSynced(true);
            showToast("success", "Буфер обмена синхронизирован");
        } catch (err) {
            logger.warn("session", "Clipboard sync failed", err);
            showToast("error", "Не удалось синхронизировать буфер обмена");
        }
    };

    const handleCopyEndpoint = async () => {
        try {
            await navigator.clipboard.writeText(remoteUrls.wsUrl);
            showToast("success", "Адрес VNC-клиента скопирован");
        } catch (err) {
            logger.warn("session", "Endpoint copy failed", err);
            showToast("error", "Не удалось скопировать адрес подключения");
        }
    };

    const currentScale = scale === "auto" ? "Auto-fit" : `${scale}%`;
    const overviewVisible = showOverview || connectionStatus !== "connected";

    return (
        <div className={`${styles.container} ${isFullscreen ? styles.fullscreen : ""}`}>
            <div className={styles.toolbar}>
                <div className={styles.meta}>
                    <span className={styles.hostBadge}>{username}@{host}</span>
                    <span className={styles.displayBadge}>Display :{displayId ?? "?"} · {endpointSummary}</span>
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
                        <span className={styles.qualText}>{modeDetails.label}</span>
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
                                const next = s === "auto" ? 100 : s === 100 ? 90 : s === 90 ? 75 : s === 75 ? 50 : "auto";
                                if (rfbRef.current) rfbRef.current.scaleViewport = next === "auto";
                                return next;
                            });
                        }}
                    >
                        🔍 Scale: {currentScale}
                    </button>
                    <button
                        className={styles.toolButton}
                        onClick={() => setConnectionMode(prev => prev === "performance" ? "balanced" : prev === "balanced" ? "clarity" : "performance")}
                    >
                        🧭 Mode: {modeDetails.label}
                    </button>
                    <button
                        className={`${styles.toolButton} ${showOverview ? styles.toolButtonActive : ""}`}
                        onClick={() => setShowOverview(prev => !prev)}
                    >
                        ℹ️ Session info
                    </button>
                    <button
                        className={styles.toolButton}
                        onClick={handleCopyEndpoint}
                    >
                        🔗 Copy endpoint
                    </button>
                    <button
                        className={styles.toolButton}
                        onClick={() => {
                            if (!document.fullscreenElement) {
                                containerRef.current?.requestFullscreen().catch(err => {
                                    showToast("error", "Полноэкранный режим недоступен", err.message);
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

            <div className={styles.content}>
                {overviewVisible && (
                <div className={styles.sessionOverview}>
                    <div className={styles.overviewCard}>
                        <div className={styles.overviewHeader}>
                            <div>
                                <div className={styles.overviewKicker}>Remote access tunnel</div>
                                <h3 className={styles.overviewTitle}>{connectionHealth.title}</h3>
                            </div>
                            <span className={`${styles.healthBadge} ${styles[connectionHealth.tone]}`}>
                                {connectionStatus.toUpperCase()}
                            </span>
                        </div>
                        <p className={styles.overviewText}>
                            {connectionStatus === "error" && errorMessage ? errorMessage : connectionHealth.detail}
                        </p>
                        <div className={styles.detailGrid}>
                            <div className={styles.detailItem}>
                                <span>Uptime</span>
                                <strong>{formatDuration(sessionSeconds)}</strong>
                            </div>
                            <div className={styles.detailItem}>
                                <span>WebSocket</span>
                                <strong>{remoteUrls.wsUrl}</strong>
                            </div>
                            <div className={styles.detailItem}>
                                <span>Upload API</span>
                                <strong>{buildUploadUrl(API_BASE_URL, "file")}</strong>
                            </div>
                            <div className={styles.detailItem}>
                                <span>Mode</span>
                                <strong>{modeDetails.badge}</strong>
                            </div>
                        </div>
                    </div>

                    <div className={styles.chromeCard}>
                        <div className={styles.chromeTitle}>Connection mode</div>
                        <div className={styles.modeSwitcher}>
                            {(["performance", "balanced", "clarity"] as ConnectionMode[]).map(mode => {
                                const details = getConnectionModeDetails(mode);
                                const active = mode === connectionMode;
                                return (
                                    <button
                                        key={mode}
                                        className={`${styles.modeButton} ${active ? styles.modeButtonActive : ""}`}
                                        onClick={() => handleSelectMode(mode)}
                                    >
                                        <span>{details.label}</span>
                                        <small>{details.description}</small>
                                    </button>
                                );
                            })}
                        </div>
                        <div className={styles.chromeActions}>
                            <button className={styles.secondaryAction} onClick={handleRetry}>
                                Reconnect
                            </button>
                            <button className={styles.secondaryAction} onClick={() => setScale(modeDetails.recommendedScale === "fit" ? "auto" : modeDetails.recommendedScale)}>
                                Apply recommended scale
                            </button>
                        </div>
                    </div>
                </div>
                )}

                {isCaptured && (
                    <div className={styles.captureHint}>
                        Захват ввода активен. Нажмите <b>Правый Ctrl</b> для выхода.
                    </div>
                )}

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
                                            const res = await fetch(buildUploadUrl(API_BASE_URL, file.name), {
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

            <div className={styles.clipboardBar}>
                <span className={styles.clipLabel}>Clipboard Status:</span>
                <input 
                    type="text" 
                    value={clipboardText} 
                    onChange={(e) => setClipboardText(e.target.value)} 
                    className={styles.clipInput}
                />
                <button className={styles.syncButton} onClick={handleCopyClipboard}>
                    {clipboardSynced ? "Clipboard synced" : "Sync to Local Clipboard"}
                </button>
            </div>
        </div>
    );
};
