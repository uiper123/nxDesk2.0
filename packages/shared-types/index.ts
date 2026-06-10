// Generated types will be written here by Specta
export type ConnectionConfig = {
  host: string;
  port: number;
  username: string;
  auth_method: AuthMethod;
};

export type AuthMethod = 
  | { Password: string }
  | { PrivateKeyPath: string }
  | "Agent";

export type SessionInfo = {
  id: string;
  username: string;
  display_id: number;
  start_time: number;
};

export type SessionStatus = "Active" | "Idle" | "Disconnected";

export type VideoSettings = {
  width: number;
  height: number;
  fps: number;
  target_bitrate_kbps: number;
};

export type MouseButton = "None" | "Left" | "Right" | "Middle";
