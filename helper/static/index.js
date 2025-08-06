// DOM elements
const statusElem = document.getElementById("status");
const serverIp = document.getElementById("serverIp");
const connectionStatus = document.getElementById("connectionStatus");

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
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(2))} ${sizes[i]}`;
}

function formatPercent(value) {
  return `${value.toFixed(0)}%`;
}

// API calls
async function startServer() {
  await executeWithAuth(async () => {
    try {
      startBtn.disabled = true;
      const response = await makeAuthenticatedRequest(
        "/api/start",
        AUTH_PASSWORD_KEY,
        {
          signal: AbortSignal.timeout(5000),
        }
      );
      const text = await response.text();

      if (response.ok) {
        showStatus(`Server started: ${text}`);
      } else if (response.status == 503) {
        showStatus("Failed to start server: server unavailable", true);
      } else {
        showStatus(`Failed to start server: ${text}`, true);
      }
    } catch (error) {
      if (error.message === "INVALID_PASSWORD") {
        throw error;
      }
      showStatus(`Error starting server: ${error.message}`, true);
    } finally {
      startBtn.disabled = false;
    }
  }, AUTH_PASSWORD_KEY);
}

async function stopServer() {
  await executeWithAuth(
    async () => {
      try {
        stopBtn.disabled = true;
        const response = await makeAuthenticatedRequest(
          "/api/stop",
          STOP_PASSWORD_KEY,
          {
            signal: AbortSignal.timeout(5000),
          }
        );
        const text = await response.text();

        if (response.ok) {
          showStatus(`Server stopped: ${text}`);
        } else if (response.status == 503) {
          showStatus("Failed to stop server: server unavailable", true);
        } else {
          showStatus(`Failed to stop server: ${text}`, true);
        }
      } catch (error) {
        if (error.message === "INVALID_PASSWORD") {
          throw error;
        }
        showStatus(`Error stopping server: ${error.message}`, true);
      } finally {
        stopBtn.disabled = false;
      }
    },
    STOP_PASSWORD_KEY,
    "Password Required - Stop Server"
  );
}

async function wakeServer() {
  await executeWithAuth(async () => {
    try {
      wakeBtn.disabled = true;
      const response = await makeAuthenticatedRequest(
        "/api/wake",
        AUTH_PASSWORD_KEY,
        {
          signal: AbortSignal.timeout(5000),
        }
      );
      const text = await response.text();

      if (response.ok) {
        showStatus(`Woke: ${text}`);
      } else if (response.status == 503) {
        showStatus("Failed to wake server: server unavailable", true);
      } else {
        showStatus(`Failed to wake server: ${text}`, true);
      }
    } catch (error) {
      if (error.message === "INVALID_PASSWORD") {
        throw error;
      }
      showStatus(`Error waking server: ${error.message}`, true);
    } finally {
      wakeBtn.disabled = false;
    }
  }, AUTH_PASSWORD_KEY);
}

async function getServerIp() {
  await executeWithAuth(async () => {
    try {
      ipBtn.disabled = true;
      const response = await makeAuthenticatedRequest(
        "/api/ip",
        AUTH_PASSWORD_KEY
      );

      if (response.ok) {
        const ip = await response.text();
        serverIp.innerHTML = `<div class="status success"><span class="noselect">Server IP: </span>${ip.trim()}</div>`;
      } else {
        const error = await response.text();
        serverIp.innerHTML = `<div class="status error">Failed to get IP: ${error}</div>`;
      }
    } catch (error) {
      if (error.message === "INVALID_PASSWORD") {
        throw error;
      }
      serverIp.innerHTML = `<div class="status error">Error getting IP: ${error.message}</div>`;
    } finally {
      ipBtn.disabled = false;
    }
  }, AUTH_PASSWORD_KEY);
}

const statsTimeout = 1000;
let lastStatsMsg = 0;
let statsConnected = false;

setInterval(() => {
  if (!statsConnected) return;

  const lastMsg = Math.round((performance.now() - lastStatsMsg) / 1000);
  if (lastMsg >= 3) {
    connectionStatus.innerText = `Connected to stats (last update ${lastMsg}s ago)`;
    connectionStatus.className = "connection-status connecting";
  } else if (connectionStatus.innerText != "Connected to stats") {
    connectionStatus.innerText = `Connected to stats`;
    connectionStatus.className = "connection-status connected";
  }
}, 1000);

/**
 * @type {WebSocket | null}
 */
let statsSocket = null;

// WebSocket connection for stats
function connectStats() {
  try {
    connectionStatus.innerText = "Connecting to stats..";
    connectionStatus.className = "connection-status connecting";
    statsSocket = new WebSocket("/api/stats");

    statsSocket.onopen = () => {
      lastStatsMsg = performance.now();
      statsConnected = true;
      connectionStatus.innerText = "Connected to stats";
      connectionStatus.className = "connection-status connected";
    };

    statsSocket.onclose = () => {
      statsConnected = false;
      connectionStatus.innerText = `Disconnected from stats`;
      connectionStatus.className = "connection-status disconnected";

      // Reconnect
      setTimeout(connectStats, statsTimeout);
    };

    statsSocket.onerror = (error) => {
      console.error("WebSocket error:", error);
    };

    statsSocket.onmessage = (event) => {
      lastStatsMsg = performance.now();
      try {
        updateStatsDisplay(JSON.parse(event.data));
      } catch (error) {
        console.error("Error processing stats:", error);
      }
    };
  } catch (error) {
    statsConnected = false;
    console.error("Failed to connect to stats WebSocket:", error);
    setTimeout(connectStats, statsTimeout);
  }
}

connectStats();

// Stats elements
const systemRam = document.getElementById("systemRam");
const cpuCores = document.getElementById("cpuCores");
const cpuTotal = document.getElementById("cpuTotal");
const serverRam = document.getElementById("serverRam");
const serverCpu = document.getElementById("serverCpu");
const serverDisk = document.getElementById("serverDisk");

function updateStatsDisplay(data) {
  const system_ram = data.system_ram_free + data.system_ram_used;
  const system_ram_free = formatBytes(data.system_ram_free);
  const system_ram_free_percent = (
    (data.system_ram_free / system_ram) *
    100
  ).toFixed(2);
  const system_ram_used = formatBytes(data.system_ram_used);
  const system_ram_used_percent = (
    (data.system_ram_used / system_ram) *
    100
  ).toFixed(2);
  systemRam.innerHTML = `<div class="green">Free: ${system_ram_free} (${system_ram_free_percent}%)</div>
  <div class="red">Used: ${system_ram_used} (${system_ram_used_percent}%)</div>`;

  if (cpuCores.children.length == 0) {
    cpuCores.innerHTML = "";
    for (const usage of data.system_cpu_usage) {
      const core = document.createElement("div");
      core.className = "cpu-core";
      core.innerText = formatPercent(usage);
      cpuCores.appendChild(core);
    }
  } else {
    let i = 0;
    for (const usage of data.system_cpu_usage) {
      cpuCores.children[i].innerText = formatPercent(usage);
      i += 1;
    }
  }
  const system_cpu_total =
    data.system_cpu_usage.reduce((sum, usage) => sum + usage, 0) /
    data.system_cpu_usage.length;
  if (system_cpu_total < 50) {
    cpuTotal.innerHTML = `System CPU Usage per Core <span class="normal">(<span class="green">${Math.round(
      system_cpu_total
    )}%</span> total)</span>`;
  } else if (system_cpu_total < 80) {
    cpuTotal.innerHTML = `System CPU Usage per Core <span class="normal">(<span class="yellow">${Math.round(
      system_cpu_total
    )}%</span> total)</span>`;
  } else {
    cpuTotal.innerHTML = `System CPU Usage per Core <span class="normal">(<span class="red">${Math.round(
      system_cpu_total
    )}%</span> total)</span>`;
  }

  if (data.server_cpu_usage) {
    const server_cpu_usage = Math.round(
      data.server_cpu_usage / data.system_cpu_usage.length
    );
    serverCpu.innerText = `${Math.round(server_cpu_usage)}%`;
  } else {
    serverCpu.innerText = "-";
  }

  if (data.server_ram_usage) {
    const server_ram_usage = formatBytes(data.server_ram_usage);
    const server_system_ram = (
      (data.server_ram_usage / system_ram) *
      100
    ).toFixed(2);
    serverRam.innerText = `${server_ram_usage} (${server_system_ram}% of system)`;
  } else {
    serverRam.innerText = "-";
  }

  if (data.server_disk_usage) {
    let server_disk_usage = formatBytes(data.server_disk_usage);
    serverDisk.innerText = `${server_disk_usage} / sec`;
  } else {
    serverDisk.innerText = "-";
  }
}

const startBtn = document.getElementById("startBtn");
const stopBtn = document.getElementById("stopBtn");
const ipBtn = document.getElementById("ipBtn");
const wakeBtn = document.getElementById("wakeBtn");

// Event listeners
startBtn.onclick = startServer;
stopBtn.onclick = stopServer;
ipBtn.onclick = getServerIp;
wakeBtn.onclick = wakeServer;

serverIp.onclick = async (ev) => {
  const ip = ev.target.innerText.split(": ");
  if (ip[1]) {
    await navigator.clipboard.writeText(ip[1]);

    const notification = document.createElement("div");
    notification.className = "copy-notification";
    notification.textContent = "copied to clipboard";

    notification.style.left = `${ev.clientX}px`;
    notification.style.top = `${ev.clientY - 12}px`;

    document.body.appendChild(notification);

    // Remove the notification after animation completes
    setTimeout(() => {
      if (notification.parentNode) {
        notification.parentNode.removeChild(notification);
      }
    }, 1000);
  }
};

const consoleElement = document.getElementById("console");

function addConsoleMessage(message) {
  const atBottom =
    consoleElement.scrollTop >=
    consoleElement.scrollHeight - consoleElement.clientHeight - 5;

  consoleElement.innerText += `${message}\n`;

  if (atBottom) {
    consoleElement.scrollTop = consoleElement.scrollHeight;
  }

  // limit console history to 500 lines
  const lines = consoleElement.innerText.split("\n");
  if (lines.length > 500) {
    consoleElement.innerText = lines.slice(-500).join("\n");
  }
}

fetch("/api/ping").then((resp) => {
  if (!resp.ok) {
    addConsoleMessage("computer does not seem to be awake");
  }
});

/**
 * @type {WebSocket | null}
 */
let consoleSocket = null;
let consoleFirstConnect = true;
const consoleStatusElement = document.getElementById("consoleStatus");

function updateConsoleStatus(message, status = "disconnected") {
  consoleStatusElement.textContent = message;
  consoleStatusElement.className = `console-status ${status}`;
}

function connectConsole() {
  try {
    updateConsoleStatus("Connecting to console...", "connecting");
    consoleSocket = new WebSocket("/api/console");

    consoleSocket.onopen = async () => {
      if (consoleFirstConnect) {
        updateConsoleStatus("Connected to console", "connected");
        await fetch("/api/list");
        consoleFirstConnect = false;
      } else {
        updateConsoleStatus("Reconnected!", "connected");
        setTimeout(() => {
          updateConsoleStatus("Connected to console", "connected");
        }, 1000);
      }
    };

    consoleSocket.onclose = () => {
      updateConsoleStatus("Disconnected from console", "disconnected");
      // Reconnect
      setTimeout(connectConsole, statsTimeout);
    };

    consoleSocket.onerror = (error) => {
      console.error(`WebSocket error: ${error}`);
      updateConsoleStatus("Console connection error", "disconnected");
    };

    consoleSocket.onmessage = (event) => {
      addConsoleMessage(event.data);
    };
  } catch (error) {
    console.error(`Failed to connect to stats WebSocket: ${error}`);
    updateConsoleStatus("Failed to connect to console", "disconnected");
    setTimeout(connectConsole, statsTimeout);
  }
}

// Initialize console status
updateConsoleStatus("Console not connected", "disconnected");
connectConsole();

// Password management
const AUTH_PASSWORD_KEY = "auth_password";
const STOP_PASSWORD_KEY = "stop_password";

function getStoredPassword(key) {
  return localStorage.getItem(key);
}

function setStoredPassword(key, password) {
  localStorage.setItem(key, password);
}

function clearStoredPassword(key) {
  localStorage.removeItem(key);
}

function createPasswordPopup(title, onSubmit, onCancel) {
  // Create overlay
  const overlay = document.createElement("div");
  overlay.className = "password-overlay";

  // Create popup
  const popup = document.createElement("div");
  popup.className = "password-popup";

  popup.innerHTML = `
    <h3>${title}</h3>
    <input type="password" id="passwordInput" placeholder="Enter password">
    <div class="button-group">
      <button id="cancelBtn" class="cancel-btn">Cancel</button>
      <button id="submitBtn" class="submit-btn">Submit</button>
    </div>
  `;

  overlay.appendChild(popup);
  document.body.appendChild(overlay);

  const passwordInput = popup.querySelector("#passwordInput");
  const submitBtn = popup.querySelector("#submitBtn");
  const cancelBtn = popup.querySelector("#cancelBtn");

  // Focus input
  passwordInput.focus();

  // Handle submit
  const handleSubmit = () => {
    const password = passwordInput.value.trim();
    if (password) {
      document.body.removeChild(overlay);
      onSubmit(password);
    }
  };

  // Handle cancel
  const handleCancel = () => {
    document.body.removeChild(overlay);
    if (onCancel) onCancel();
  };

  // Event listeners
  submitBtn.onclick = handleSubmit;
  cancelBtn.onclick = handleCancel;
  passwordInput.onkeypress = (e) => {
    if (e.key === "Enter") handleSubmit();
    if (e.key === "Escape") handleCancel();
  };

  // Close on overlay click
  overlay.onclick = (e) => {
    if (e.target === overlay) handleCancel();
  };
}

async function makeAuthenticatedRequest(url, passwordKey) {
  const password = getStoredPassword(passwordKey);

  if (!password) {
    throw new Error("NO_PASSWORD");
  }

  const response = await fetch(url, {
    headers: {
      token: password,
    },
  });

  // If unauthorized, clear stored password
  if (response.status === 401) {
    clearStoredPassword(passwordKey);
    throw new Error("INVALID_PASSWORD");
  }

  return response;
}

async function executeWithAuth(action, passwordKey) {
  try {
    // Try with stored password first
    const storedPassword = getStoredPassword(passwordKey);
    if (storedPassword) {
      try {
        await action();
        return;
      } catch (error) {
        if (error.message === "INVALID_PASSWORD") {
          // Password is invalid, continue to prompt
        } else {
          throw error;
        }
      }
    }

    // Prompt for password
    createPasswordPopup("need password", async (password) => {
      try {
        setStoredPassword(passwordKey, password);
        await action();
      } catch (error) {
        if (error.message === "INVALID_PASSWORD") {
          clearStoredPassword(passwordKey);
          showStatus("invalid password..", true);
        } else {
          throw error;
        }
      }
    });
  } catch (error) {
    if (
      error.message !== "NO_PASSWORD" &&
      error.message !== "INVALID_PASSWORD"
    ) {
      throw error;
    }
  }
}
