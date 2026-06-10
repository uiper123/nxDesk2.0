import React, { useState, useEffect } from "react";
import styles from "./AdminPanel.module.css";
import { apiService, ActiveSession } from "../../services/api";
import { useToast } from "../Toast";
import { logger } from "../../services/logger";

export const AdminPanel: React.FC = () => {
    const { showToast } = useToast();
    const [sessions, setSessions] = useState<ActiveSession[]>([]);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState("");

    const fetchSessions = async () => {
        try {
            const data = await apiService.getActiveSessions();
            setSessions(data);
            setError("");
        } catch (err) {
            setError("Failed to load sessions");
            logger.error("AdminPanel", "Error fetching sessions:", err);
        } finally {
            setLoading(false);
        }
    };

    useEffect(() => {
        fetchSessions();
        // Обновляем каждые 5 секунд
        const interval = setInterval(fetchSessions, 5000);
        return () => clearInterval(interval);
    }, []);

    const handleTerminate = async (sessionId: string, username: string) => {
        const confirmTerm = window.confirm(`Are you sure you want to forcibly terminate session for user ${username}?`);
        if (confirmTerm) {
            try {
                await apiService.terminateSession(sessionId);
                setSessions((prev) => prev.filter((s) => s.id !== sessionId));
                showToast("success", `Сессия пользователя ${username} принудительно завершена`);
            } catch (err) {
                showToast("error", "Не удалось завершить сессию", String(err));
                logger.error("AdminPanel", "Error terminating session:", err);
            }
        }
    };

    if (loading) {
        return (
            <div className={styles.container}>
                <div className={styles.header}>
                    <h2 className={styles.title}>Admin Control Center</h2>
                </div>
                <div style={{ padding: "2rem", textAlign: "center" }}>Loading sessions...</div>
            </div>
        );
    }

    if (error) {
        return (
            <div className={styles.container}>
                <div className={styles.header}>
                    <h2 className={styles.title}>Admin Control Center</h2>
                </div>
                <div style={{ padding: "2rem", textAlign: "center", color: "red" }}>{error}</div>
            </div>
        );
    }

    return (
        <div className={styles.container}>
            <div className={styles.header}>
                <h2 className={styles.title}>Admin Control Center</h2>
                <span className={styles.warningBadge}>System Administration Privileged Mode</span>
            </div>

            <div className={styles.section}>
                <h3 className={styles.sectionTitle}>Active Server Sessions</h3>

                <div className={styles.tableWrapper}>
                    <table className={styles.table}>
                        <thead>
                            <tr>
                                <th>User</th>
                                <th>Display</th>
                                <th>Start Time</th>
                                <th>CPU Load</th>
                                <th>RAM usage</th>
                                <th>Actions</th>
                            </tr>
                        </thead>
                        <tbody>
                            {sessions.length === 0 ? (
                                <tr>
                                    <td colSpan={6} style={{ textAlign: "center", padding: "2rem" }}>
                                        No active sessions
                                    </td>
                                </tr>
                            ) : (
                                sessions.map((session) => (
                                    <tr key={session.id}>
                                        <td>
                                            <div className={styles.userCell}>
                                                <span className={styles.userSymbol}>👤</span>
                                                <span>{session.username}</span>
                                            </div>
                                        </td>
                                        <td>
                                            <span className={styles.displayBadge}>:{session.display_id}</span>
                                        </td>
                                        <td>{session.start_time}</td>
                                        <td>
                                            <span className={session.cpu_usage > 10 ? styles.highLoad : ""}>
                                                {session.cpu_usage}%
                                            </span>
                                        </td>
                                        <td>{session.mem_usage} MB</td>
                                        <td>
                                            <button
                                                className={styles.terminateButton}
                                                onClick={() => handleTerminate(session.id, session.username)}
                                            >
                                                Terminate Session
                                            </button>
                                        </td>
                                    </tr>
                                ))
                            )}
                        </tbody>
                    </table>
                </div>
            </div>
        </div>
    );
};
