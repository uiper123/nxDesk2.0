import React, { useState, useEffect } from "react";
import styles from "./Settings.module.css";
import { apiService } from "../../services/api";
import { useToast } from "../Toast";
import { logger } from "../../services/logger";

export const Settings: React.FC = () => {
    const { showToast } = useToast();
    const [quality, setQuality] = useState("auto");
    const [encoder, setEncoder] = useState("vaapi");
    const [fps, setFps] = useState(30);
    const [audio, setAudio] = useState(false);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState("");
    const [saving, setSaving] = useState(false);

    useEffect(() => {
        const fetchSettings = async () => {
            try {
                const data = await apiService.getSettings();
                setQuality(data.quality);
                setEncoder(data.encoder);
                setFps(data.fps);
                setAudio(data.audio);
            } catch (err) {
                setError("Failed to load settings");
                logger.error("settings", "Error fetching settings", err);
            } finally {
                setLoading(false);
            }
        };

        fetchSettings();
    }, []);

    const handleSave = async () => {
        setSaving(true);
        try {
            await apiService.updateSettings({
                quality,
                encoder,
                fps,
                audio,
            });
            showToast("success", "Settings saved", "New parameters will apply to the next session.");
        } catch (err) {
            showToast("error", "Failed to save settings", err instanceof Error ? err.message : undefined);
            logger.error("settings", "Error saving settings", err);
        } finally {
            setSaving(false);
        }
    };

    if (loading) {
        return (
            <div className={styles.container}>
                <h2 className={styles.title}>System Settings</h2>
                <div style={{ padding: "2rem", textAlign: "center" }}>Loading settings...</div>
            </div>
        );
    }

    if (error) {
        return (
            <div className={styles.container}>
                <h2 className={styles.title}>System Settings</h2>
                <div style={{ padding: "2rem", textAlign: "center", color: "red" }}>{error}</div>
            </div>
        );
    }

    return (
        <div className={styles.container}>
            <h2 className={styles.title}>System Settings</h2>
            
            <div className={styles.section}>
                <h3 className={styles.sectionTitle}>Video & Quality</h3>
                
                <div className={styles.settingRow}>
                    <div className={styles.settingInfo}>
                        <label>Stream Quality</label>
                        <span>Adjust resolution and compression level.</span>
                    </div>
                    <select value={quality} onChange={(e) => setQuality(e.target.value)}>
                        <option value="auto">Auto (Adaptive)</option>
                        <option value="high">High (1080p 6000kbps)</option>
                        <option value="medium">Medium (720p 3000kbps)</option>
                        <option value="low">Low (480p 1000kbps)</option>
                    </select>
                </div>

                <div className={styles.settingRow}>
                    <div className={styles.settingInfo}>
                        <label>Target Framerate</label>
                        <span>Higher values require more CPU / bandwidth.</span>
                    </div>
                    <select value={fps} onChange={(e) => setFps(Number(e.target.value))}>
                        <option value={15}>15 FPS</option>
                        <option value={30}>30 FPS</option>
                        <option value={60}>60 FPS</option>
                    </select>
                </div>

                <div className={styles.settingRow}>
                    <div className={styles.settingInfo}>
                        <label>Hardware Acceleration</label>
                        <span>Use VAAPI (Intel/AMD/NVIDIA) where available.</span>
                    </div>
                    <select value={encoder} onChange={(e) => setEncoder(e.target.value)}>
                        <option value="vaapi">GStreamer + VAAPI H.264</option>
                        <option value="software">Software Fallback (OpenH264)</option>
                    </select>
                </div>
            </div>

            <div className={styles.section}>
                <h3 className={styles.sectionTitle}>Audio Settings</h3>
                <div className={styles.settingRow}>
                    <div className={styles.settingInfo}>
                        <label>Enable Remote Audio</label>
                        <span>Listen to audio from the remote Astra Linux host.</span>
                    </div>
                    <input 
                        type="checkbox" 
                        checked={audio} 
                        onChange={(e) => setAudio(e.target.checked)} 
                        className={styles.checkbox}
                    />
                </div>
            </div>

            <button className={styles.saveButton} onClick={handleSave} disabled={saving}>
                {saving ? "Saving..." : "Save Changes"}
            </button>
        </div>
    );
};
