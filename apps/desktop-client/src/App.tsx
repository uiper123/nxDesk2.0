import { useState } from "react";
import { Login } from "./components/Login";
import { HostList } from "./components/HostList";
import { ConnectionCard } from "./components/ConnectionCard";
import { ActiveSession } from "./components/ActiveSession";
import { Settings } from "./components/Settings";
import { Logs } from "./components/Logs";
import { AdminPanel } from "./components/AdminPanel";
import { Dashboard } from "./components/Dashboard";
import { ApiHealth } from "./components/ApiHealth";
import "./App.css";

type View = "dashboard" | "hosts" | "admin" | "logs" | "settings";

function App() {
  const [isAuthenticated, setIsAuthenticated] = useState(false);
  const [sessionState, setSessionState] = useState<"disconnected" | "connecting" | "connected">("disconnected");
  
  // Connection details
  const [currentHost, setCurrentHost] = useState("");
  const [currentPort, setCurrentPort] = useState(22);
  const [currentUser, setCurrentUser] = useState("");
  const [currentDisplayId, setCurrentDisplayId] = useState<number | undefined>(undefined);
  const [currentView, setCurrentView] = useState<View>("dashboard");

  const handleLoginSuccess = (_host: string, _port: number, username: string) => {
    setCurrentUser(username);
    setIsAuthenticated(true);
  };

  const handleSelectHost = (hostIp: string, port: number, username: string, displayId?: number) => {
    setCurrentHost(hostIp);
    setCurrentPort(port);
    setCurrentUser(username);
    setCurrentDisplayId(displayId);
    setSessionState("connecting");
  };

  const handleDisconnect = () => {
    setSessionState("disconnected");
  };

  // 1. Unauthenticated -> Show Login Screen
  if (!isAuthenticated) {
    return <Login onLoginSuccess={handleLoginSuccess} />;
  }

  // 2. Connecting State -> Show Connection Progress Card
  if (sessionState === "connecting") {
    return (
      <ConnectionCard 
        host={currentHost} 
        username={currentUser} 
        onConnected={() => setSessionState("connected")}
        onCancel={() => setSessionState("disconnected")}
      />
    );
  }

  // 3. Connected State -> Show Active Session Canvas
  if (sessionState === "connected") {
    return (
      <ActiveSession 
        host={currentHost} 
        port={currentPort}
        username={currentUser}
        displayId={currentDisplayId}
        onDisconnect={handleDisconnect}
      />
    );
  }

  // 4. Authenticated & Disconnected -> Show Main Workstation Layout with Sidebar
  return (
    <div className="app-workspace">
      {/* Sidebar Navigation */}
      <aside className="sidebar-nav">
        <div className="brand">
          <span className="brand-logo">🛡️</span>
          <h2>TTGTiSO-Desk</h2>
        </div>

        <nav className="nav-menu">
          <button 
            className={`nav-item ${currentView === "dashboard" ? "active" : ""}`}
            onClick={() => setCurrentView("dashboard")}
          >
            📊 Dashboard
          </button>
          <button 
            className={`nav-item ${currentView === "hosts" ? "active" : ""}`}
            onClick={() => setCurrentView("hosts")}
          >
            🖥️ Hosts Registry
          </button>
          <button 
            className={`nav-item ${currentView === "admin" ? "active" : ""}`}
            onClick={() => setCurrentView("admin")}
          >
            ⚙️ Admin Panel
          </button>
          <button 
            className={`nav-item ${currentView === "logs" ? "active" : ""}`}
            onClick={() => setCurrentView("logs")}
          >
            📋 Audit Logs
          </button>
          <button 
            className={`nav-item ${currentView === "settings" ? "active" : ""}`}
            onClick={() => setCurrentView("settings")}
          >
            🔧 Settings
          </button>
        </nav>

        <div className="sidebar-footer">
          <ApiHealth />
          <div className="user-profile">
            <span className="avatar">👤</span>
            <div className="user-info">
              <span className="username">{currentUser}</span>
              <span className="role">Operator</span>
            </div>
          </div>
          <button className="logout-btn" onClick={() => setIsAuthenticated(false)}>
            Lock Console
          </button>
        </div>
      </aside>

      {/* Main Workspace Content */}
      <main className="workspace-content">
        {currentView === "dashboard" && (
          <Dashboard onNavigate={(view) => setCurrentView(view as View)} />
        )}
        {currentView === "hosts" && <HostList onSelectHost={handleSelectHost} />}
        {currentView === "admin" && <AdminPanel />}
        {currentView === "logs" && <Logs />}
        {currentView === "settings" && <Settings />}
      </main>
    </div>
  );
}

export default App;
