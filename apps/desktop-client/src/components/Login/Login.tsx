import React, { useState } from "react";
import styles from "./Login.module.css";
import { apiService } from "../../services/api";
import { logger } from "../../services/logger";

interface LoginProps {
    onLoginSuccess: (host: string, port: number, username: string, role: string, token: string) => void;
}

export const Login: React.FC<LoginProps> = ({ onLoginSuccess }) => {
    const [username, setUsername] = useState("admin");
    const [password, setPassword] = useState("");
    const [loading, setLoading] = useState(false);
    const [error, setError] = useState("");

    const handleSubmit = async (e: React.FormEvent) => {
        e.preventDefault();
        setError("");
        setLoading(true);

        try {
            const response = await apiService.login({
                host: "",
                port: 0,
                username,
                password,
            });

            if (response.success) {
                onLoginSuccess("", 0, username, response.user?.role || "Operator", response.user?.token || "");
            } else {
                setError(response.message || "Authentication failed");
            }
        } catch (err) {
            setError("Cannot connect to API server. Please ensure it's running.");
            logger.error("Login", "Login request failed", err);
        } finally {
            setLoading(false);
        }
    };

    return (
        <div className={styles.loginContainer}>
            <form onSubmit={handleSubmit} className={styles.loginCard}>
                <div className={styles.header}>
                    <h1 className={styles.title}>TTGTiSO-Desk</h1>
                    <p className={styles.subtitle}>Secure Remote Access Client for Astra Linux</p>
                </div>

                {error && <div className={styles.errorMessage}>{error}</div>}

                <div className={styles.inputGroup}>
                    <label htmlFor="login-username">Username</label>
                    <input 
                        id="login-username"
                        type="text" 
                        value={username} 
                        onChange={(e) => setUsername(e.target.value)} 
                        required
                    />
                </div>

                <div className={styles.inputGroup}>
                    <label htmlFor="login-password">Password / Private Key passphrase</label>
                    <input 
                        id="login-password"
                        type="password" 
                        value={password} 
                        onChange={(e) => setPassword(e.target.value)} 
                        placeholder="••••••••"
                        required
                    />
                </div>

                <button type="submit" className={styles.loginButton} disabled={loading}>
                    {loading ? <span className={styles.spinner}></span> : "Establish Secure Session"}
                </button>

                <div className={styles.footer}>
                    <span>Closed Network Offline Mode Active</span>
                    <span className={styles.badge}>Cross-Platform (Linux/Windows)</span>
                </div>
            </form>
        </div>
    );
};
