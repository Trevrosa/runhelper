// DOM elements
const startBtn = document.getElementById("startBtn");
const stopBtn = document.getElementById("stopBtn");
const ipBtn = document.getElementById("ipBtn");
const statusElem = document.getElementById("status");
const serverIp = document.getElementById("serverIp");
const connectionStatus = document.getElementById("connectionStatus");

// Stats elements
const systemRam = document.getElementById("systemRam");
const cpuCores = document.getElementById("cpuCores");
const serverRam = document.getElementById("serverRam");
const serverCpu = document.getElementById("serverCpu");
const serverDisk = document.getElementById("serverDisk");

let statsSocket = null;

// Utility functions
function showStatus(message, isError = false) {
  statusElem.innerHTML = `<div class="status ${
    isError ? "error" : "success"
  }">${message}</div>`;
  setTimeout(() => (statusElem.innerHTML = ""), 5000);
}

function formatBytes(bytes) {
  if (bytes === 0) return "0 B";
  const k = 1024;
  const sizes = ["B", "KB", "MB", "GB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + " " + sizes[i];
}

function formatPercent(value) {
  return `${value.toFixed(1)}%`;
}

// API calls
async function startServer() {
  try {
    startBtn.disabled = true;
    const response = await fetch("/api/run");
    const text = await response.text();

    if (response.ok) {
      showStatus(`Server started: ${text}`);
    } else {
      showStatus(`Failed to start server: ${text}`, true);
    }
  } catch (error) {
    showStatus(`Error starting server: ${error.message}`, true);
  } finally {
    startBtn.disabled = false;
  }
}

async function stopServer() {
  try {
    stopBtn.disabled = true;
    const response = await fetch("/api/stop");
    const text = await response.text();

    if (response.ok) {
      showStatus(`Server stopped: ${text}`);
    } else {
      showStatus(`Failed to stop server: ${text}`, true);
    }
  } catch (error) {
    showStatus(`Error stopping server: ${error.message}`, true);
  } finally {
    stopBtn.disabled = false;
  }
}

async function getServerIp() {
  try {
    ipBtn.disabled = true;
    const response = await fetch("/api/ip");

    if (response.ok) {
      const ip = await response.text();
      serverIp.innerHTML = `<div class="status success">Server IP: ${ip.trim()}</div>`;
    } else {
      const error = await response.text();
      serverIp.innerHTML = `<div class="status error">Failed to get IP: ${error}</div>`;
    }
  } catch (error) {
    serverIp.innerHTML = `<div class="status error">Error getting IP: ${error.message}</div>`;
  } finally {
    ipBtn.disabled = false;
  }
}

// WebSocket connection for stats
function connectStats() {
  try {
    statsSocket = new WebSocket("/api/stats");

    statsSocket.onopen = () => {
      connectionstatusElem.textContent = "Connected to stats";
      connectionstatusElem.className = "connection-status connected";
    };

    statsSocket.onclose = () => {
      connectionstatusElem.textContent = "Disconnected from stats";
      connectionstatusElem.className = "connection-status disconnected";

      // Reconnect after 3 seconds
      setTimeout(connectStats, 3000);
    };

    statsSocket.onerror = (error) => {
      console.error("WebSocket error:", error);
    };

    statsSocket.onmessage = (event) => {
      try {
        // The data comes as debug format, so we'll display it as-is for now
        // In a real implementation, you'd parse the actual Stats struct
        const data = event.data;

        // For now, just show raw data in console and update with placeholder
        console.log("Stats received:", data);

        // You would parse the actual stats here
        // For demonstration, showing placeholder updates
        updateStatsDisplay(data);
      } catch (error) {
        console.error("Error processing stats:", error);
      }
    };
  } catch (error) {
    console.error("Failed to connect to stats WebSocket:", error);
    setTimeout(connectStats, 3000);
  }
}

function updateStatsDisplay(rawData) {
  // Since the stats come as debug format text, we'll show a simplified version
  // In a real implementation, you'd parse the actual Stats struct

  systemRam.textContent = "Receiving data...";
  cpuCores.textContent = "Receiving data...";
  serverRam.textContent = rawData.includes("server_ram_usage: Some")
    ? "Active"
    : "N/A";
  serverCpu.textContent = rawData.includes("server_cpu_usage: Some")
    ? "Active"
    : "N/A";
  serverDisk.textContent = rawData.includes("server_disk_usage: Some")
    ? "Active"
    : "N/A";
}

// Event listeners
startBtn.addEventListener("click", startServer);
stopBtn.addEventListener("click", stopServer);
ipBtn.addEventListener("click", getServerIp);

// Initialize
document.addEventListener("DOMContentLoaded", () => {
  connectStats();
});
