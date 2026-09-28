const $ = (id) => document.getElementById(id);

const COOKIE_NAMES = ["ssid", "sub", "csid", "clid", "tdid"];

function formatRelative(iso) {
  if (!iso) return "nunca";
  const then = new Date(iso).getTime();
  const now = Date.now();
  const s = Math.round((now - then) / 1000);
  if (s < 60) return `hace ${s}s`;
  if (s < 3600) return `hace ${Math.round(s / 60)}min`;
  if (s < 86400) return `hace ${Math.round(s / 3600)}h`;
  return `hace ${Math.round(s / 86400)}d`;
}

async function renderStatus() {
  const { lastSyncAt, lastSyncResult } = await chrome.storage.local.get(["lastSyncAt", "lastSyncResult"]);
  const dot = $("dot");
  const statusText = $("statusText");

  if (!lastSyncAt) {
    dot.className = "dot";
    statusText.textContent = "No sincronizado";
    return;
  }
  if (lastSyncResult?.ok) {
    dot.className = "dot ok";
    statusText.textContent = `OK · ${formatRelative(lastSyncAt)}`;
  } else {
    dot.className = "dot err";
    statusText.textContent = `Error · ${formatRelative(lastSyncAt)}`;
  }
}

async function renderCookies() {
  const preview = await chrome.runtime.sendMessage({ type: "get-cookies-preview" });
  const ul = $("cookieList");
  ul.innerHTML = "";
  for (const name of COOKIE_NAMES) {
    const li = document.createElement("li");
    const val = preview?.[name];
    li.className = val ? "" : "missing";
    li.innerHTML = `<span>${name}</span><strong>${val || "—"}</strong>`;
    ul.appendChild(li);
  }
}

async function renderConfig() {
  const { backendUrl, pairToken } = await chrome.storage.local.get(["backendUrl", "pairToken"]);
  $("backendUrl").value = backendUrl || "https://val.migueltaibo.com";
  $("pairToken").value = pairToken || "";
}

async function saveConfig() {
  await chrome.storage.local.set({
    backendUrl: $("backendUrl").value.trim(),
    pairToken: $("pairToken").value.trim(),
  });
  const hint = $("hint");
  hint.className = "hint ok";
  hint.textContent = "Configuración guardada";
  setTimeout(() => (hint.textContent = ""), 2000);
}

async function syncNow() {
  const btn = $("syncNow");
  const hint = $("hint");
  btn.disabled = true;
  btn.textContent = "Sincronizando…";
  hint.className = "hint";
  hint.textContent = "";

  const result = await chrome.runtime.sendMessage({ type: "sync-now" });
  if (result?.ok) {
    hint.className = "hint ok";
    hint.textContent = `Sesión válida hasta ${new Date(result.expiresAt).toLocaleDateString()}`;
  } else {
    hint.className = "hint err";
    hint.textContent = result?.error || "Error desconocido";
  }
  btn.disabled = false;
  btn.textContent = "Sync now";
  await renderStatus();
}

document.addEventListener("DOMContentLoaded", async () => {
  $("version").textContent = `v${chrome.runtime.getManifest().version}`;
  await renderConfig();
  await renderStatus();
  await renderCookies();
  $("saveConfig").addEventListener("click", saveConfig);
  $("syncNow").addEventListener("click", syncNow);
});
