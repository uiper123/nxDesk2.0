import React, { useEffect, useState } from "react";
import styles from "./HostList.module.css";
import { apiService, Host, ActiveSession } from "../../services/api";
import { useToast } from "../Toast";
import { logger } from "../../services/logger";

interface HostListProps {
    onSelectHost: (hostIp: string, port: number, username: string, displayId?: number) => void;
}

export const HostList: React.FC<HostListProps> = ({ onSelectHost }) => {
    const { showToast } = useToast();
    const [hosts, setHosts] = useState<Host[]>([]);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState("");

    const [selectedHost, setSelectedHost] = useState<Host | null>(null);
    const [activeSessions, setActiveSessions] = useState<ActiveSession[]>([]);
    const [systemUsers, setSystemUsers] = useState<string[]>([]);
    const [newUsername, setNewUsername] = useState("");
    const [modalLoading, setModalLoading] = useState(false);
    const [modalError, setModalError] = useState("");

    // Add Host state
    const [showAddHost, setShowAddHost] = useState(false);
    const [newHostName, setNewHostName] = useState("");
    const [newHostIp, setNewHostIp] = useState("");
    const [newHostPort, setNewHostPort] = useState(2222);
    const [addHostLoading, setAddHostLoading] = useState(false);
    const [addHostError, setAddHostError] = useState("");

    // Scan Hosts state
    const [showScanHosts, setShowScanHosts] = useState(false);
    const [discoveredHosts, setDiscoveredHosts] = useState<Host[]>([]);
    const [scanLoading, setScanLoading] = useState(false);

    // Search / filter / sort state
    const [searchQuery, setSearchQuery] = useState("");
    const [statusFilter, setStatusFilter] = useState<"all" | "online" | "busy" | "offline">("all");
    const [sortBy, setSortBy] = useState<"name" | "status" | "sessions">("status");

    const statusRank: Record<string, number> = { online: 0, busy: 1, offline: 2 };
    const visibleHosts = hosts
        .filter(h => statusFilter === "all" || h.status === statusFilter)
        .filter(h => {
            const q = searchQuery.trim().toLowerCase();
            if (!q) return true;
            return h.name.toLowerCase().includes(q) || h.ip.toLowerCase().includes(q);
        })
        .sort((a, b) => {
            if (sortBy === "name") return a.name.localeCompare(b.name);
            if (sortBy === "sessions") return b.active_sessions - a.active_sessions;
            return (statusRank[a.status] ?? 3) - (statusRank[b.status] ?? 3) || a.name.localeCompare(b.name);
        });

    useEffect(() => {
        const fetchHosts = async () => {
            try {
                const data = await apiService.getHosts();
                setHosts(data);
            } catch (err) {
                setError("Failed to load hosts");
                logger.error("HostList", "Error fetching hosts:", err);
            } finally {
                setLoading(false);
            }
        };

        fetchHosts();
        const interval = setInterval(fetchHosts, 10000);
        return () => clearInterval(interval);
    }, []);

    const handleHostCardClick = async (host: Host) => {
        if (host.status === "offline") return;
        
        setSelectedHost(host);
        setModalLoading(true);
        setModalError("");
        setNewUsername("");

        try {
            const sessions = await apiService.getActiveSessions();
            const hostSessions = sessions.filter(
                s => s.host_ip === host.ip || (host.ip === "127.0.0.1" && s.host_ip === "localhost") || (host.ip === "localhost" && s.host_ip === "127.0.0.1")
            );
            setActiveSessions(hostSessions);

            try {
                const sysUsers = await apiService.getSystemUsers(host.ip);
                setSystemUsers(sysUsers);
            } catch (uErr) {
                logger.error("HostList", "Failed to fetch system users", uErr);
                setSystemUsers([]);
            }
        } catch (err) {
            logger.error("HostList", "Error fetching sessions for host:", err);
            setModalError("Failed to fetch active sessions");
        } finally {
            setModalLoading(false);
        }
    };

    const handleSelectSession = (session: ActiveSession) => {
        if (!selectedHost) return;
        onSelectHost(selectedHost.ip, selectedHost.port, session.username, session.display_id);
        setSelectedHost(null);
    };

    const handleStartNewSession = async () => {
        if (!selectedHost || !newUsername.trim()) return;
        
        setModalLoading(true);
        setModalError("");
        
        try {
            const res = await apiService.startSession({
                host: selectedHost.ip,
                port: selectedHost.port,
                username: newUsername.trim(),
                password: ""
            });

            if (res.success) {
                const username = newUsername.trim();
                const sessions = await apiService.getActiveSessions();
                const session = sessions.find(
                    s => (
                        s.host_ip === selectedHost.ip ||
                        (selectedHost.ip === "127.0.0.1" && s.host_ip === "localhost") ||
                        (selectedHost.ip === "localhost" && s.host_ip === "127.0.0.1")
                    ) && s.username === username
                );
                onSelectHost(selectedHost.ip, selectedHost.port, username, session?.display_id);
                setSelectedHost(null);
            } else {
                setModalError(res.message || "Failed to start session");
            }
        } catch (err: any) {
            logger.error("HostList", "Error starting new session:", err);
            setModalError(err.message || "Network error while starting session");
        } finally {
            setModalLoading(false);
        }
    };

    const handleAddHost = async () => {
        if (!newHostName.trim() || !newHostIp.trim()) return;
        setAddHostLoading(true);
        setAddHostError("");
        try {
            const res = await apiService.addHost({
                name: newHostName.trim(),
                ip: newHostIp.trim(),
                port: newHostPort,
            });
            if (res.success) {
                setShowAddHost(false);
                setNewHostName("");
                setNewHostIp("");
                setNewHostPort(2222);
                // force refresh
                const data = await apiService.getHosts();
                setHosts(data);
            } else {
                setAddHostError(res.message || "Failed to add host");
            }
        } catch (err: any) {
            setAddHostError(err.message || "Network error");
        } finally {
            setAddHostLoading(false);
        }
    };

    const handleScanHosts = async () => {
        setShowScanHosts(true);
        setScanLoading(true);
        try {
            const data = await apiService.getDiscoveredHosts();
            setDiscoveredHosts(data);
        } catch (err) {
            logger.error("HostList", "Failed to scan hosts", err);
        } finally {
            setScanLoading(false);
        }
    };

    const handleAddDiscovered = async (h: Host) => {
        try {
            const res = await apiService.addHost({
                name: h.name,
                ip: h.ip,
                port: h.port,
            });
            if (res.success) {
                const data = await apiService.getHosts();
                setHosts(data);
                showToast("success", `Хост ${h.name} добавлен`);
            } else {
                showToast("error", "Ошибка", res.message);
            }
        } catch (err: any) {
            showToast("error", "Ошибка", err.message);
        }
    };

    if (loading) {
        return (
            <div className={styles.container}>
                <div className={styles.header}>
                    <h2>Реестр управляемых хостов</h2>
                </div>
                <div style={{ padding: "2rem", textAlign: "center" }}>Загрузка хостов...</div>
            </div>
        );
    }

    if (error) {
        return (
            <div className={styles.container}>
                <div className={styles.header}>
                    <h2>Реестр управляемых хостов</h2>
                </div>
                <div style={{ padding: "2rem", textAlign: "center", color: "#f7768e" }}>{error}</div>
            </div>
        );
    }

    return (
        <div className={styles.container}>
            <div className={styles.header}>
                <div className={styles.headerLeft}>
                    <h2>Реестр управляемых хостов</h2>
                    <span className={styles.count}>{visibleHosts.length} из {hosts.length}</span>
                </div>
                <div className={styles.headerActions}>
                    <button className={styles.secondaryButton} onClick={handleScanHosts}>
                        🔍 Сканировать сеть
                    </button>
                    <button className={styles.secondaryButton} onClick={() => setShowAddHost(true)}>
                        + Добавить хост
                    </button>
                </div>
            </div>

            <div className={styles.toolbar}>
                <input
                    type="search"
                    className={styles.searchInput}
                    placeholder="Поиск по имени или IP…"
                    value={searchQuery}
                    onChange={(e) => setSearchQuery(e.target.value)}
                    aria-label="Поиск хостов"
                />
                <select
                    className={styles.filterSelect}
                    value={statusFilter}
                    onChange={(e) => setStatusFilter(e.target.value as typeof statusFilter)}
                    aria-label="Фильтр по статусу"
                >
                    <option value="all">Все статусы</option>
                    <option value="online">Online</option>
                    <option value="busy">Busy</option>
                    <option value="offline">Offline</option>
                </select>
                <select
                    className={styles.filterSelect}
                    value={sortBy}
                    onChange={(e) => setSortBy(e.target.value as typeof sortBy)}
                    aria-label="Сортировка"
                >
                    <option value="status">По статусу</option>
                    <option value="name">По имени</option>
                    <option value="sessions">По сессиям</option>
                </select>
            </div>

            <div className={styles.list}>
                {visibleHosts.length === 0 ? (
                    <div className={styles.emptyState}>
                        {hosts.length === 0
                            ? "Хосты ещё не добавлены. Добавьте хост вручную или просканируйте сеть."
                            : "Ничего не найдено по заданным условиям."}
                    </div>
                ) : (
                visibleHosts.map((host) => (
                    <div 
                        key={host.id} 
                        className={`${styles.card} ${styles[host.status]}`}
                        onClick={() => handleHostCardClick(host)}
                    >
                        <div className={styles.statusIndicator}></div>
                        <div className={styles.info}>
                            <h3 className={styles.name}>{host.name}</h3>
                            <span className={styles.ip}>{host.ip}</span>
                        </div>

                        <div className={styles.details}>
                            <span className={styles.os}>{host.operating_system}</span>
                            <span className={styles.sessions}>
                                {host.active_sessions} активных сессий
                            </span>
                        </div>

                        <button 
                            className={styles.connectButton}
                            disabled={host.status === "offline"}
                        >
                            {host.status === "offline" ? "Недоступен" : "Подключиться"}
                        </button>
                    </div>
                ))
                )}
            </div>

            {selectedHost && (
                <div className={styles.modalOverlay} onClick={() => setSelectedHost(null)}>
                    <div className={styles.modalContent} onClick={(e) => e.stopPropagation()}>
                        <div className={styles.modalHeader}>
                            <h3>Подключение к {selectedHost.name}</h3>
                            <button className={styles.closeButton} onClick={() => setSelectedHost(null)}>✕</button>
                        </div>

                        {modalLoading && activeSessions.length === 0 ? (
                            <div className={styles.loadingText}>Загрузка информации о сессиях...</div>
                        ) : (
                            <>
                                <div className={styles.modalSection}>
                                    <h4>Активные сессии на хосте</h4>
                                    <div className={styles.sessionList}>
                                        {activeSessions.length === 0 ? (
                                            <div className={styles.noSessions}>
                                                Нет активных сессий пользователей.
                                            </div>
                                        ) : (
                                            activeSessions.map((session) => (
                                                <div 
                                                    key={session.id} 
                                                    className={styles.sessionItem}
                                                    onClick={() => handleSelectSession(session)}
                                                >
                                                    <span className={styles.sessionUser}>👤 {session.username}</span>
                                                    <span className={styles.sessionTime}>Дисплей: {session.display_id}</span>
                                                </div>
                                            ))
                                        )}
                                    </div>
                                </div>

                                <div className={styles.modalSection}>
                                    <h4>Запуск новой сессии</h4>
                                    
                                    {systemUsers.length > 0 && (
                                        <div style={{ display: 'flex', flexDirection: 'column', gap: '8px', marginBottom: '15px' }}>
                                            <div style={{ fontSize: '12px', color: '#9aa5ce', marginBottom: '4px' }}>Доступные пользователи:</div>
                                            {systemUsers.map(u => {
                                                const isRunning = activeSessions.some(s => s.username === u);
                                                return (
                                                    <div key={u} style={{ 
                                                        display: 'flex', justifyContent: 'space-between', alignItems: 'center', 
                                                        background: '#1a1b26', padding: '8px 12px', borderRadius: '4px' 
                                                    }}>
                                                        <span style={{ color: '#c0caf5' }}>👤 {u}</span>
                                                        <button 
                                                            className={styles.actionButton} 
                                                            onClick={() => { setNewUsername(u); setTimeout(handleStartNewSession, 0); }}
                                                            disabled={isRunning || modalLoading}
                                                            style={{ 
                                                                padding: '6px 12px', 
                                                                background: isRunning ? '#292e42' : '#2d334b', 
                                                                color: isRunning ? '#565f89' : '#c0caf5',
                                                                opacity: isRunning || modalLoading ? 0.5 : 1
                                                            }}
                                                        >
                                                            {isRunning ? 'Запущено' : 'Запустить'}
                                                        </button>
                                                    </div>
                                                );
                                            })}
                                        </div>
                                    )}

                                    <div style={{ fontSize: '12px', color: '#9aa5ce', marginBottom: '8px' }}>Или запустить с произвольным именем:</div>
                                    <div className={styles.inputGroup} style={{ flexDirection: 'row', gap: '10px' }}>
                                        <input 
                                            type="text" 
                                            placeholder="Имя пользователя (например, user1)" 
                                            value={newUsername}
                                            onChange={(e) => setNewUsername(e.target.value)}
                                            disabled={modalLoading}
                                            style={{ flex: 1 }}
                                        />
                                        <button 
                                            className={styles.actionButton}
                                            onClick={handleStartNewSession}
                                            disabled={modalLoading || !newUsername.trim()}
                                        >
                                            Запустить
                                        </button>
                                    </div>
                                    {modalError && (
                                        <div className={styles.errorText}>{modalError}</div>
                                    )}
                                </div>
                            </>
                        )}
                    </div>
                </div>
            )}

            {showAddHost && (
                <div className={styles.modalOverlay} onClick={() => setShowAddHost(false)}>
                    <div className={styles.modalContent} onClick={(e) => e.stopPropagation()}>
                        <div className={styles.modalHeader}>
                            <h3>Добавление хоста</h3>
                            <button className={styles.closeButton} onClick={() => setShowAddHost(false)}>✕</button>
                        </div>
                        <div className={styles.modalSection}>
                            <div className={styles.inputGroup} style={{ flexDirection: 'column', gap: '10px' }}>
                                <input 
                                    type="text" 
                                    placeholder="Имя хоста (например, Server1)" 
                                    value={newHostName}
                                    onChange={(e) => setNewHostName(e.target.value)}
                                    disabled={addHostLoading}
                                />
                                <input 
                                    type="text" 
                                    placeholder="IP адрес (например, 192.168.1.100)" 
                                    value={newHostIp}
                                    onChange={(e) => setNewHostIp(e.target.value)}
                                    disabled={addHostLoading}
                                />
                                <input 
                                    type="number" 
                                    placeholder="Порт агента (обычно 2222)" 
                                    value={newHostPort}
                                    onChange={(e) => setNewHostPort(Number(e.target.value))}
                                    disabled={addHostLoading}
                                />
                                <button 
                                    className={styles.actionButton}
                                    onClick={handleAddHost}
                                    disabled={addHostLoading || !newHostName.trim() || !newHostIp.trim()}
                                    style={{ marginTop: '10px', width: '100%' }}
                                >
                                    Добавить
                                </button>
                            </div>
                            {addHostError && (
                                <div className={styles.errorText}>{addHostError}</div>
                            )}
                        </div>
                    </div>
                </div>
            )}

            {showScanHosts && (
                <div className={styles.modalOverlay} onClick={() => setShowScanHosts(false)}>
                    <div className={styles.modalContent} onClick={(e) => e.stopPropagation()} style={{ maxWidth: '600px' }}>
                        <div className={styles.modalHeader}>
                            <h3>Сканирование сети</h3>
                            <button className={styles.closeButton} onClick={() => setShowScanHosts(false)}>✕</button>
                        </div>
                        <div className={styles.modalSection}>
                            <p style={{ color: '#9aa5ce', marginBottom: '15px' }}>
                                Ожидание UDP beacon пакетов от агентов в локальной сети...
                            </p>
                            
                            {scanLoading ? (
                                <div className={styles.spinner} style={{ margin: '20px auto' }}></div>
                            ) : (
                                <div style={{ display: 'flex', flexDirection: 'column', gap: '10px' }}>
                                    {discoveredHosts.length === 0 ? (
                                        <div style={{ textAlign: 'center', color: '#565f89', padding: '20px' }}>
                                            Не найдено новых агентов.
                                        </div>
                                    ) : (
                                        discoveredHosts.map(h => {
                                            const isAlreadyAdded = hosts.some(reg => reg.ip === h.ip);
                                            return (
                                                <div key={h.ip} style={{ 
                                                    display: 'flex', justifyContent: 'space-between', alignItems: 'center', 
                                                    background: '#1a1b26', padding: '10px 15px', borderRadius: '6px' 
                                                }}>
                                                    <div>
                                                        <div style={{ fontWeight: 'bold', color: '#c0caf5' }}>{h.name}</div>
                                                        <div style={{ fontSize: '12px', color: '#7dcfff' }}>{h.ip}:{h.port}</div>
                                                    </div>
                                                    <button 
                                                        onClick={() => handleAddDiscovered(h)}
                                                        disabled={isAlreadyAdded}
                                                        style={{ 
                                                            background: isAlreadyAdded ? '#292e42' : '#7aa2f7', 
                                                            color: isAlreadyAdded ? '#565f89' : '#1a1b26',
                                                            border: 'none', padding: '6px 12px', borderRadius: '4px',
                                                            cursor: isAlreadyAdded ? 'not-allowed' : 'pointer',
                                                            fontWeight: 'bold'
                                                        }}
                                                    >
                                                        {isAlreadyAdded ? 'Уже добавлен' : 'Добавить'}
                                                    </button>
                                                </div>
                                            );
                                        })
                                    )}
                                    <button 
                                        onClick={handleScanHosts}
                                        style={{ 
                                            background: '#3b4261', color: '#c0caf5', border: 'none', 
                                            padding: '8px', borderRadius: '4px', cursor: 'pointer', marginTop: '10px' 
                                        }}
                                    >
                                        🔄 Обновить список
                                    </button>
                                </div>
                            )}
                        </div>
                    </div>
                </div>
            )}
        </div>
    );
};
