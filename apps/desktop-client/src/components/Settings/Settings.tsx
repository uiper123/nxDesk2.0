import React, { useEffect, useState } from "react";
import styles from "./Settings.module.css";
import { apiService } from "../../services/api";
import { useToast } from "../Toast";
import { logger } from "../../services/logger";
import { invoke } from "@tauri-apps/api/core";
import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

export const Settings: React.FC = () => {
    const { showToast } = useToast();
    const [quality, setQuality] = useState("auto");
    const [encoder, setEncoder] = useState("vaapi");
    const [fps, setFps] = useState(30);
    const [audio, setAudio] = useState(false);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState("");
    const [saving, setSaving] = useState(false);
    const [updateStatus, setUpdateStatus] = useState<string>("Checking for updates...");
    const [currentVersion, setCurrentVersion] = useState<string>("0.1.0");
    const [updateBusy, setUpdateBusy] = useState(false);
    const [sshPublicKey, setSshPublicKey] = useState("");
    const [sshPublicKeyPath, setSshPublicKeyPath] = useState("");
    const [sshPrivateKeyPath, setSshPrivateKeyPath] = useState("");
    const [sshKeyBusy, setSshKeyBusy] = useState(false);

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

        const loadVersion = async () => {
            try {
                const version = await invoke<string>("get_app_version");
                setCurrentVersion(version);
            } catch (err) {
                logger.error("settings", "Error getting app version", err);
            }
        };

        const loadSshIdentity = async () => {
            try {
                const identity = await apiService.ensureSshIdentity();
                setSshPublicKey(identity.public_key);
                setSshPublicKeyPath(identity.public_key_path);
                setSshPrivateKeyPath(identity.private_key_path);
            } catch (err) {
                logger.error("settings", "Error ensuring SSH identity", err);
            }
        };

        loadVersion();
        loadSshIdentity();
    }, []);

    const handleCheckUpdates = async () => {
        setUpdateBusy(true);
        setUpdateStatus("Checking for updates...");
        try {
            const update = await check();
            if (!update) {
                setUpdateStatus("No updates available.");
                return;
            }

            setUpdateStatus(`Found ${update.version}. Downloading and installing...`);
            await update.downloadAndInstall((event) => {
                if (event.event === "Started") {
                    setUpdateStatus(`Downloading ${event.data.contentLength} bytes...`);
                }
                if (event.event === "Progress") {
                    setUpdateStatus(`Downloading: ${event.data.chunkLength} bytes received...`);
                }
                if (event.event === "Finished") {
                    setUpdateStatus("Installing update...");
                }
            });
            setUpdateStatus("Update installed. Restarting...");
            await relaunch();
        } catch (err) {
            setUpdateStatus(err instanceof Error ? err.message : "Update check failed");
            logger.error("settings", "Update check failed", err);
        } finally {
            setUpdateBusy(false);
        }
    };

    const handleRegenerateSshKey = async () => {
        setSshKeyBusy(true);
        try {
            const identity = await apiService.regenerateSshIdentity();
            setSshPublicKey(identity.public_key);
            setSshPublicKeyPath(identity.public_key_path);
            setSshPrivateKeyPath(identity.private_key_path);
            showToast("success", "SSH key regenerated", "Скопируйте public key и добавьте его на нужный хост.");
        } catch (err) {
            showToast("error", "Failed to regenerate SSH key", err instanceof Error ? err.message : undefined);
            logger.error("settings", "Error regenerating SSH key", err);
        } finally {
            setSshKeyBusy(false);
        }
    };

    const copySshPublicKey = async () => {
        if (!sshPublicKey) return;
        try {
            await navigator.clipboard.writeText(sshPublicKey);
            showToast("success", "SSH public key copied");
        } catch (err) {
            showToast("error", "Failed to copy SSH key", err instanceof Error ? err.message : undefined);
        }
    };

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
                <div className={styles.stateText}>Loading settings...</div>
            </div>
        );
    }

    if (error) {
        return (
            <div className={styles.container}>
                <h2 className={styles.title}>System Settings</h2>
                <div className={`${styles.stateText} ${styles.stateError}`}>{error}</div>
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
                <h3 className={styles.sectionTitle}>SSH Access Key</h3>
                <div className={styles.settingRow}>
                    <div className={styles.settingInfo}>
                        <label>Public Key</label>
                        <span>{sshPublicKeyPath || "Generating..."}</span>
                    </div>
                    <button type="button" className={styles.saveButton} onClick={copySshPublicKey} disabled={!sshPublicKey}>
                        Copy
                    </button>
                </div>
                <div className={styles.settingRow}>
                    <div className={styles.settingInfo}>
                        <label>Current Key</label>
                        <span className={styles.keyBox}>{sshPublicKey || "Generating SSH key on this client..."}</span>
                    </div>
                </div>
                <div className={styles.settingRow}>
                    <div className={styles.settingInfo}>
                        <label>Private Key Path</label>
                        <span>{sshPrivateKeyPath || "—"}</span>
                    </div>
                    <button type="button" className={styles.saveButton} onClick={handleRegenerateSshKey} disabled={sshKeyBusy}>
                        {sshKeyBusy ? "Regenerating..." : "Regenerate"}
                    </button>
                </div>
            </div>

            <div className={styles.section}>
                <h3 className={styles.sectionTitle}>Application Updates</h3>
                <div className={styles.settingRow}>
                    <div className={styles.settingInfo}>
                        <label>Current Version</label>
                        <span>{currentVersion}</span>
                    </div>
                    <button type="button" className={styles.saveButton} onClick={handleCheckUpdates} disabled={updateBusy}>
                        {updateBusy ? "Checking..." : "Check for Updates"}
                    </button>
                </div>
                <div className={styles.stateText}>{updateStatus}</div>
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
