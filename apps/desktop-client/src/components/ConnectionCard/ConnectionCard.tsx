import React, { useCallback, useEffect, useRef, useState } from "react";
import styles from "./ConnectionCard.module.css";
import { apiService } from "../../services/api";
import { logger } from "../../services/logger";

interface ConnectionCardProps {
    host: string;
    username: string;
    onConnected: () => void;
    onCancel: () => void;
}

type StepStatus = "pending" | "running" | "done" | "failed";

interface StepDefinition {
    label: string;
    run: (ctx: { host: string; username: string }) => Promise<string | void>;
}

const MIN_STEP_VISIBLE_MS = 350;

const STEPS: StepDefinition[] = [
    {
        label: "Проверка доступности API-сервера...",
        run: async () => {
            const health = await apiService.healthCheck();
            if (!health.ok) {
                throw new Error("API-сервер недоступен. Проверьте, что api-server запущен.");
            }
            return `API ответил за ${health.latencyMs} мс`;
        },
    },
    {
        label: "Поиск хоста в реестре...",
        run: async ({ host }) => {
            const hosts = await apiService.getHosts();
            const target = hosts.find(
                h =>
                    h.ip === host ||
                    (host === "127.0.0.1" && h.ip === "localhost") ||
                    (host === "localhost" && h.ip === "127.0.0.1"),
            );
            if (!target) {
                throw new Error(`Хост ${host} не найден в реестре.`);
            }
            if (target.status === "offline") {
                throw new Error(`Хост ${target.name} (${host}) сейчас offline.`);
            }
            return `${target.name} · ${target.operating_system}`;
        },
    },
    {
        label: "Согласование пользовательской сессии...",
        run: async ({ host, username }) => {
            const sessions = await apiService.getActiveSessions();
            const session = sessions.find(
                s =>
                    (s.host_ip === host ||
                        (host === "127.0.0.1" && s.host_ip === "localhost") ||
                        (host === "localhost" && s.host_ip === "127.0.0.1")) &&
                    s.username === username,
            );
            return session
                ? `Найдена сессия на дисплее :${session.display_id}`
                : "Сессия будет поднята на системном дисплее";
        },
    },
    {
        label: "Инициализация видеоканала и захвата ввода...",
        run: async () => {
            await new Promise(resolve => setTimeout(resolve, MIN_STEP_VISIBLE_MS));
        },
    },
];

export const ConnectionCard: React.FC<ConnectionCardProps> = ({ host, username, onConnected, onCancel }) => {
    const [stepStatuses, setStepStatuses] = useState<StepStatus[]>(() => STEPS.map(() => "pending"));
    const [stepNotes, setStepNotes] = useState<(string | undefined)[]>(() => STEPS.map(() => undefined));
    const [failure, setFailure] = useState<string | null>(null);
    const [attempt, setAttempt] = useState(0);
    const cancelledRef = useRef(false);

    useEffect(() => {
        cancelledRef.current = false;
        return () => {
            cancelledRef.current = true;
        };
    }, []);

    useEffect(() => {
        let active = true;

        const execute = async () => {
            setFailure(null);
            setStepStatuses(STEPS.map(() => "pending"));
            setStepNotes(STEPS.map(() => undefined));

            for (let i = 0; i < STEPS.length; i++) {
                if (!active || cancelledRef.current) return;

                setStepStatuses(prev => prev.map((s, idx) => (idx === i ? "running" : s)));
                const startedAt = performance.now();

                try {
                    const note = await STEPS[i].run({ host, username });
                    const elapsed = performance.now() - startedAt;
                    if (elapsed < MIN_STEP_VISIBLE_MS) {
                        await new Promise(resolve => setTimeout(resolve, MIN_STEP_VISIBLE_MS - elapsed));
                    }
                    if (!active || cancelledRef.current) return;

                    setStepStatuses(prev => prev.map((s, idx) => (idx === i ? "done" : s)));
                    if (note) {
                        setStepNotes(prev => prev.map((n, idx) => (idx === i ? note : n)));
                    }
                } catch (err: any) {
                    if (!active || cancelledRef.current) return;
                    logger.error("connection", `Pre-flight step failed: ${STEPS[i].label}`, err);
                    setStepStatuses(prev => prev.map((s, idx) => (idx === i ? "failed" : s)));
                    setFailure(err?.message || "Неизвестная ошибка при подготовке сессии.");
                    return;
                }
            }

            if (active && !cancelledRef.current) {
                setTimeout(() => {
                    if (!cancelledRef.current) onConnected();
                }, 350);
            }
        };

        execute();
        return () => {
            active = false;
        };
    }, [host, username, attempt, onConnected]);

    const handleRetry = useCallback(() => setAttempt(prev => prev + 1), []);

    const doneCount = stepStatuses.filter(s => s === "done").length;
    const progressPercent = Math.min((doneCount / STEPS.length) * 100, 100);
    const visibleSteps = STEPS.map((step, idx) => ({ step, status: stepStatuses[idx], note: stepNotes[idx] }))
        .filter(({ status }) => status !== "pending");

    return (
        <div className={styles.container}>
            <div className={styles.card}>
                <div className={`${styles.pulseLogo} ${failure ? styles.pulseFailed : ""}`}>
                    <div className={styles.pulseInner}></div>
                </div>

                <h2 className={styles.title}>{failure ? "Сбой подключения" : "Establishing Connection"}</h2>
                <p className={styles.subtitle}>{username}@{host}</p>

                <div className={styles.progressContainer}>
                    <div
                        className={`${styles.progressBar} ${failure ? styles.progressFailed : ""}`}
                        style={{ width: `${failure ? 100 : progressPercent}%` }}
                    ></div>
                </div>

                <div className={styles.logBox} role="log" aria-live="polite">
                    {visibleSteps.map(({ step, status, note }, idx) => (
                        <div
                            key={idx}
                            className={`${styles.logLine} ${
                                status === "running" ? styles.active : status === "failed" ? styles.failed : styles.done
                            }`}
                        >
                            <span className={styles.indicator}>
                                {status === "done" ? "✓" : status === "failed" ? "✕" : "●"}
                            </span>
                            <span>
                                {step.label}
                                {note && <span className={styles.stepNote}> — {note}</span>}
                            </span>
                        </div>
                    ))}
                    {failure && <div className={`${styles.logLine} ${styles.failed}`}>{failure}</div>}
                </div>

                <div className={styles.actions}>
                    {failure && (
                        <button className={styles.retryButton} onClick={handleRetry}>
                            Повторить проверку
                        </button>
                    )}
                    <button className={styles.cancelButton} onClick={onCancel}>
                        {failure ? "Назад к хостам" : "Cancel Connection"}
                    </button>
                </div>
            </div>
        </div>
    );
};
