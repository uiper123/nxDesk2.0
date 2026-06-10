import React, { useCallback, useEffect, useState } from "react";
import styles from "./Dashboard.module.css";
import { apiService, Host, ActiveSession, LogEntry } from "../../services/api";
import { logger } from "../../services/logger";

interface DashboardProps {
    onNavigate: (view: "hosts" | "admin" | "logs" | "settings") => void;
}

interface Snapshot {
    hosts: Host[];
    sessions: ActiveSession[];
    logs: LogEntry[];
}

const REFRESH_INTERVAL_MS = 10_000;

export const Dashboard: React.FC<DashboardProps> = ({ onNavigate }) => {
    const [snapshot, setSnapshot] = useState<Snapshot>({ hosts: [], sessions: [], logs: [] });
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState("");

    const fetchAll = useCallback(async () => {
        try {
            const [hosts, sessions, logs] = await Promise.all([
                apiService.getHosts(),
                apiService.getActiveSessions(),
                apiService.getLogs(),
            ]);
            setSnapshot({ hosts, sessions, logs });
            setError("");
        } catch (err) {
            logger.error("Dashboard", "Failed to load overview data", err);
            setError("Не удалось загрузить данные обзора");
        } finally {
            setLoading(false);
        }
    }, []);

    useEffect(() => {
        fetchAll();
        const interval = setInterval(fetchAll, REFRESH_INTERVAL_MS);
        return () => clearInterval(interval);
    }, [fetchAll]);

    const online = snapshot.hosts.filter(h => h.status === "online").length;
    const busy = snapshot.hosts.filter(h => h.status === "busy").length;
    const offline = snapshot.hosts.filter(h => h.status === "offline").length;
    const recentLogs = snapshot.logs.slice(-8).reverse();

    if (loading) {
        return (
            <div className={styles.container}>
                <h2 className={styles.title}>Обзор системы</h2>
                <div className={styles.skeletonGrid}>
                    {[0, 1, 2, 3].map(i => <div key={i} className={styles.skeletonCard} />)}
                </div>
            </div>
        );
    }

    return (
        <div className={styles.container}>
            <div className={styles.headerRow}>
                <h2 className={styles.title}>Обзор системы</h2>
                <span className={styles.refreshHint}>автообновление каждые 10 с</span>
            </div>

            {error && <div className={styles.errorBanner}>{error}</div>}

            <div className={styles.statGrid}>
                <button className={styles.statCard} onClick={() => onNavigate("hosts")}>
                    <span className={styles.statValue}>{snapshot.hosts.length}</span>
                    <span className={styles.statLabel}>Всего хостов</span>
                    <div className={styles.statMeta}>
                        <span className={styles.dotOnline} /> {online} online
                        <span className={styles.dotBusy} /> {busy} busy
                        <span className={styles.dotOffline} /> {offline} offline
                    </div>
                </button>

                <button className={styles.statCard} onClick={() => onNavigate("admin")}>
                    <span className={styles.statValue}>{snapshot.sessions.length}</span>
                    <span className={styles.statLabel}>Активные сессии</span>
                    <div className={styles.statMeta}>
                        {snapshot.sessions.length > 0
                            ? `${new Set(snapshot.sessions.map(s => s.username)).size} пользователей`
                            : "нет активных подключений"}
                    </div>
                </button>

                <button className={styles.statCard} onClick={() => onNavigate("logs")}>
                    <span className={styles.statValue}>
                        {snapshot.logs.filter(l => l.level === "ERROR" || l.level === "WARN").length}
                    </span>
                    <span className={styles.statLabel}>Предупреждения и ошибки</span>
                    <div className={styles.statMeta}>{snapshot.logs.length} записей в журнале</div>
                </button>

                <button className={styles.statCard} onClick={() => onNavigate("settings")}>
                    <span className={styles.statValue}>
                        {snapshot.sessions.reduce((acc, s) => acc + s.cpu_usage, 0).toFixed(0)}%
                    </span>
                    <span className={styles.statLabel}>Суммарная нагрузка CPU</span>
                    <div className={styles.statMeta}>по всем сессиям</div>
                </button>
            </div>

            <div className={styles.panelGrid}>
                <section className={styles.panel}>
                    <div className={styles.panelHeader}>
                        <h3>Активные сессии</h3>
                        <button className={styles.linkButton} onClick={() => onNavigate("admin")}>
                            Управление →
                        </button>
                    </div>
                    {snapshot.sessions.length === 0 ? (
                        <div className={styles.empty}>Нет активных сессий</div>
                    ) : (
                        <ul className={styles.sessionList}>
                            {snapshot.sessions.slice(0, 5).map(s => (
                                <li key={s.id} className={styles.sessionRow}>
                                    <span className={styles.sessionUser}>{s.username}</span>
                                    <span className={styles.sessionHost}>{s.host_ip} · :{s.display_id}</span>
                                    <span className={styles.sessionLoad}>
                                        CPU {s.cpu_usage.toFixed(0)}% · RAM {s.mem_usage.toFixed(0)}%
                                    </span>
                                </li>
                            ))}
                        </ul>
                    )}
                </section>

                <section className={styles.panel}>
                    <div className={styles.panelHeader}>
                        <h3>Последние события</h3>
                        <button className={styles.linkButton} onClick={() => onNavigate("logs")}>
                            Весь журнал →
                        </button>
                    </div>
                    {recentLogs.length === 0 ? (
                        <div className={styles.empty}>Журнал пуст</div>
                    ) : (
                        <ul className={styles.logList}>
                            {recentLogs.map((log, idx) => (
                                <li key={idx} className={styles.logRow}>
                                    <span className={`${styles.logBadge} ${styles[log.level.toLowerCase()]}`}>
                                        {log.level}
                                    </span>
                                    <span className={styles.logMsg}>{log.message}</span>
                                </li>
                            ))}
                        </ul>
                    )}
                </section>
            </div>
        </div>
    );
};
