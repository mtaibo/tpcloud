const COOKIE_NAMES = ["ssid", "sub", "csid", "clid", "tdid"];
const RIOT_DOMAIN = ".riotgames.com";
const ALARM_NAME = "tpval-sync-alarm";
const SYNC_INTERVAL_MINUTES = 60 * 72; // 72h
const VERSION = chrome.runtime.getManifest().version;

async function getConfig() {
  const { backendUrl, pairToken } = await chrome.storage.local.get(["backendUrl", "pairToken"]);
  return {
    backendUrl: (backendUrl || "https://val.migueltaibo.com").replace(/\/$/, ""),
    pairToken: pairToken || "",
  };
}

async function collectCookies() {
  const cookies = {};
  for (const name of COOKIE_NAMES) {
    try {
      const c = await chrome.cookies.get({ url: `https://auth.riotgames.com/`, name });
      if (c && c.value) cookies[name] = c.value;
    } catch (e) {
      // Some cookies may be scoped to other subdomains — fall back to a domain-wide search.
    }
  }
  if (!cookies.ssid) {
    try {
      const all = await chrome.cookies.getAll({ domain: RIOT_DOMAIN });
      for (const c of all) {
        if (COOKIE_NAMES.includes(c.name) && !cookies[c.name]) {
          cookies[c.name] = c.value;
        }
      }
    } catch (e) {
      // ignore
    }
  }
  return cookies;
}

async function syncNow() {
  const { backendUrl, pairToken } = await getConfig();
  if (!pairToken) {
    return { ok: false, error: "Falta el pair token. Ábrelo en el popup." };
  }

  const cookies = await collectCookies();
  if (!cookies.ssid) {
    return { ok: false, error: "No se encontró la cookie ssid — inicia sesión en auth.riotgames.com primero." };
  }

  try {
    const res = await fetch(`${backendUrl}/api/val/account/extension/sync`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "X-TPVal-Extension-Token": pairToken,
      },
      body: JSON.stringify({ cookies, extension_version: VERSION }),
    });
    const data = await res.json().catch(() => ({}));
    if (!res.ok) {
      await recordSync({ ok: false, error: data.detail || `HTTP ${res.status}` });
      return { ok: false, error: data.detail || `HTTP ${res.status}` };
    }
    await recordSync({ ok: true, expiresAt: data.session_expires_at });
    return { ok: true, expiresAt: data.session_expires_at };
  } catch (e) {
    await recordSync({ ok: false, error: e.message });
    return { ok: false, error: e.message };
  }
}

async function recordSync(result) {
  await chrome.storage.local.set({
    lastSyncAt: new Date().toISOString(),
    lastSyncResult: result,
  });
}

chrome.runtime.onInstalled.addListener(() => {
  chrome.alarms.create(ALARM_NAME, {
    delayInMinutes: 1,
    periodInMinutes: SYNC_INTERVAL_MINUTES,
  });
});

chrome.alarms.onAlarm.addListener(async (alarm) => {
  if (alarm.name === ALARM_NAME) {
    await syncNow();
  }
});

chrome.runtime.onMessage.addListener((msg, _sender, sendResponse) => {
  if (msg && msg.type === "sync-now") {
    syncNow().then(sendResponse);
    return true;
  }
  if (msg && msg.type === "get-cookies-preview") {
    collectCookies().then((cookies) => {
      const preview = {};
      for (const [k, v] of Object.entries(cookies)) {
        preview[k] = v ? v.slice(0, 8) + "…" : null;
      }
      sendResponse(preview);
    });
    return true;
  }
});
