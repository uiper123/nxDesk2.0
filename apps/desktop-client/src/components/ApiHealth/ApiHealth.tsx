import React, { useEffect, useState } from "react";
import styles from "./ApiHealth.module.css";
import { apiService } from "../../services/api";

const CHECK_INTERVAL_MS = 15_000;

type HealthState = "checking" | "online" | "offline";

export const ApiHealth: React.FC = () => {
    const [state, setState] = useState<HealthState>("checking");
    const [latency, setLatency] = useState<number | null>(null);

    useEffect(() => {
        let cancelled = false;

        const check = async () => {
            const result = await apiService.healthCheck();
            if (cancelled) return;
            if (result.ok) {
                setState("online");
                setLatency(result.latencyMs);
            } else {
                setState("offline");
                setLatency(null);
            }
        };

        check();
        const interval = setInterval(check, CHECK_INTERVAL_MS);
        return () => {
            cancelled = true;
            clearInterval(interval);
        };
    }, []);

    return (
        <div className={`${styles.badge} ${styles[state]}`} title="Состояние api-server">
            <span className={styles.dot} />
            <span className={styles.label}>
                {state === "checking" && "Проверка API..."}
                {state === "online" && `API online · ${latency} ms`}
                {state === "offline" && "API offline"}
            </span>
        </div>
    );
};
