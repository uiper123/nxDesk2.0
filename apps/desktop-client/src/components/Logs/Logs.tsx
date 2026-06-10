import React, { useState, useEffect } from "react";
import styles from "./Logs.module.css";
import { apiService, LogEntry } from "../../services/api";
import { logger } from "../../services/logger";
import { useToast } from "../Toast";

function makeCsv(rows: LogEntry[]): string {
  const escape = (v: string) => `"${v.replace(/"/g, '""')}"`;
  const header = "timestamp,level,message";
  const lines = rows.map(r => [escape(r.timestamp), escape(r.level), escape(r.message)].join(","));
  return [header, ...lines].join("\n");
}

function downloadFile(filename: string, content: string) {
  const blob = new Blob([content], { type: "text/plain" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  a.click();
  URL.revokeObjectURL(url);
}

export const Logs: React.FC = () => {
    const { showToast } = useToast();
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

    const handleExport = (format: "json" | "csv") => {
        if (filteredLogs.length === 0) {
            showToast("info", "Журнал пуст — экспортировать нечего");
            return;
        }
        const stamp = new Date().toISOString().replace(/[:.]/g, "-");
        if (format === "json") {
            downloadFile(`audit-log-${stamp}.json`, JSON.stringify(filteredLogs, null, 2));
        } else {
            const csvContent = makeCsv(filteredLogs);
            downloadFile(`audit-log-${stamp}.csv`, csvContent);
        }
        showToast("success", `Журнал экспортирован (${format.toUpperCase()}, ${filteredLogs.length} записей)`);
    };

    if (loading) {
        return (
            <div className={styles.container}>
                <div className={styles.header}>
                    <h2 className={styles.title}>System Log Audits</h2>
                </div>
                <div className={styles.stateText}>Loading logs...</div>
            </div>
        );
    }

    if (error) {
        return (
            <div className={styles.container}>
                <div className={styles.header}>
                    <h2 className={styles.title}>System Log Audits</h2>
                </div>
                <div className={`${styles.stateText} ${styles.stateError}`}>{error}</div>
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
                    <button className={styles.exportBtn} onClick={() => handleExport("json")}>
                        JSON
                    </button>
                    <button className={styles.exportBtn} onClick={() => handleExport("csv")}>
                        CSV
                    </button>
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
