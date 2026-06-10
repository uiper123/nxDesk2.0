import React, { useEffect, useState } from "react";
import styles from "./ConnectionCard.module.css";

interface ConnectionCardProps {
    host: string;
    username: string;
    onConnected: () => void;
    onCancel: () => void;
}

const STEPS = [
    "Initializing secure SSH transport layer...",
    "Verifying host identity keys...",
    "Authenticating user credentials...",
    "Allocating virtual X11 display socket...",
    "Launching isolated Fly session manager...",
    "Initializing video pipeline & input control..."
];

export const ConnectionCard: React.FC<ConnectionCardProps> = ({ host, username, onConnected, onCancel }) => {
    const [currentStep, setCurrentStep] = useState(0);

    useEffect(() => {
        if (currentStep < STEPS.length) {
            const delay = 600 + Math.random() * 400; // randomized step delay
            const timer = setTimeout(() => {
                setCurrentStep(prev => prev + 1);
            }, delay);
            return () => clearTimeout(timer);
        } else {
            // Once all steps are complete, trigger session launch
            const timer = setTimeout(() => {
                onConnected();
            }, 500);
            return () => clearTimeout(timer);
        }
    }, [currentStep, onConnected]);

    const progressPercent = Math.min((currentStep / STEPS.length) * 100, 100);

    return (
        <div className={styles.container}>
            <div className={styles.card}>
                <div className={styles.pulseLogo}>
                    <div className={styles.pulseInner}></div>
                </div>

                <h2 className={styles.title}>Establishing Connection</h2>
                <p className={styles.subtitle}>{username}@{host}</p>

                <div className={styles.progressContainer}>
                    <div className={styles.progressBar} style={{ width: `${progressPercent}%` }}></div>
                </div>

                <div className={styles.logBox}>
                    {STEPS.slice(0, currentStep + 1).map((step, idx) => (
                        <div 
                            key={idx} 
                            className={`${styles.logLine} ${idx === currentStep ? styles.active : styles.done}`}
                        >
                            <span className={styles.indicator}>
                                {idx < currentStep ? "✓" : "●"}
                            </span>
                            {step}
                        </div>
                    ))}
                </div>

                <button className={styles.cancelButton} onClick={onCancel}>
                    Cancel Connection
                </button>
            </div>
        </div>
    );
};
