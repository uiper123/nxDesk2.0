import React, { useState, useEffect } from "react";
import styles from "./Logs.module.css";
import { apiService, LogEntry } from "../../services/api";
import { logger } from "../../services/logger";

export const Logs: React.FC = () => {
    const [filter, setFilter] = useState<"ALL" | "INFO" | "WARN" | "ERROR" | "AUDIT">("ALL");
    const [logs, setLogs] = useState<LogEntry[]>([]);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState("");

    const fetchLogs = async () => {
        try {
            const data = await apiService.getLogs();
            setLogs(data);
            setError("");
        } catch (err) {
            setError("Failed to load logs");
            logger.error("Logs", "Failed to fetch logs", err);
        } finally {
            setLoading(false);
        }
    };

    useEffect(() => {
        fetchLogs();
        // Обновляем каждые 3 секунды
        const interval = setInterval(fetchLogs, 3000);
        return () => clearInterval(interval);
    }, []);

    const filteredLogs = logs.filter(
        (l) => filter === "ALL" || l.level === filter
    );

    const handleClear = () => {
        setLogs([]);
    };

    if (loading) {
        return (
            <div className={styles.container}>
                <div className={styles.header}>
                    <h2 className={styles.title}>System Log Audits</h2>
                </div>
                <div style={{ padding: "2rem", textAlign: "center" }}>Loading logs...</div>
            </div>
        );
    }

    if (error) {
        return (
            <div className={styles.container}>
                <div className={styles.header}>
                    <h2 className={styles.title}>System Log Audits</h2>
                </div>
                <div style={{ padding: "2rem", textAlign: "center", color: "red" }}>{error}</div>
            </div>
        );
    }

    return (
        <div className={styles.container}>
            <div className={styles.header}>
                <h2 className={styles.title}>System Log Audits</h2>
                <div className={styles.actions}>
                    <select 
                        value={filter} 
                        onChange={(e) => setFilter(e.target.value as any)}
                        className={styles.select}
                    >
                        <option value="ALL">Show All</option>
                        <option value="INFO">Info</option>
                        <option value="WARN">Warnings</option>
                        <option value="ERROR">Errors</option>
                        <option value="AUDIT">Audits</option>
                    </select>
                    <button className={styles.clearBtn} onClick={handleClear}>
                        Clear
                    </button>
                </div>
            </div>

            <div className={styles.logBox}>
                {filteredLogs.length > 0 ? (
                    filteredLogs.map((log, idx) => (
                        <div key={idx} className={`${styles.logRow} ${styles[log.level.toLowerCase()]}`}>
                            <span className={styles.time}>[{log.timestamp}]</span>
                            <span className={styles.badge}>{log.level}</span>
                            <span className={styles.msg}>{log.message}</span>
                        </div>
                    ))
                ) : (
                    <div className={styles.empty}>No logs found matching filter.</div>
                )}
            </div>
        </div>
    );
};
