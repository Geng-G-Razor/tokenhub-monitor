import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

// ---- Types mirroring the Rust PackageData ----------------------------------
interface PackageData {
  id: number;
  plan_id: number;
  plan_code: string;
  plan_display_name: string;
  package_kind: string;
  status: string;
  billing_unit: string;

  total_quota: number;
  remaining_quota: number;

  used_daily: number;
  used_weekly: number;
  used_week: number;
  used_monthly: number;
  used_5h: number;

  weekly_limit: number | null;
  daily_limit: number | null;
  monthly_limit: number | null;
  wall_week_limit: number | null;
  wall_5h_limit: number | null;

  rpm_total_limit: number;
  rpm_success_limit: number;

  supported_models: string[];

  activated_at: string;
  absolute_expire_at: string;
  weekly_reset_at: string | null;
  weekly_reset_after_hours: number | null;

  source_redemption_code: string | null;

  created_at: string;
  updated_at: string;
}

// ---- Formatters -------------------------------------------------------------
/// Full number (e.g. "1,234,567") — for variable usage counts where
/// abbreviations like "1.2K" would hide meaningful changes.
const num = (n: number) => n.toLocaleString("en-US");
/// Compact number (e.g. "1.2K") — for fixed quotas that stay stable.
const compact = (n: number) =>
  n.toLocaleString("en-US", { notation: "compact", maximumFractionDigits: 1 });

const time = () =>
  new Date().toLocaleTimeString("zh-CN", {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });

/// Format an ISO datetime string to a human-readable relative or absolute date.
function formatDate(iso: string): string {
  const d = new Date(iso);
  const now = Date.now();
  const diff = d.getTime() - now;
  const absDiff = Math.abs(diff);

  if (diff < 0 && absDiff < 86400000) {
    // Past within 24 hours
    return `${Math.round(absDiff / 3600000)} 小时前`;
  }
  if (diff > 0 && diff < 86400000) {
    // Future within 24 hours
    return `${Math.round(diff / 3600000)} 小时后`;
  }
  if (diff > 0 && diff < 259200000) {
    // Future within 3 days
    return `${Math.round(diff / 86400000)} 天后`;
  }
  if (absDiff < 259200000) {
    return `${Math.round(absDiff / 86400000)} 天前`;
  }
  // Fallback: short date
  return d.toLocaleDateString("zh-CN", {
    month: "numeric",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

// ---- DOM helpers ------------------------------------------------------------
const $ = <T extends HTMLElement = HTMLElement>(id: string) =>
  document.getElementById(id) as T;

function show(view: "loading" | "login" | "dash") {
  $("view-loading").classList.toggle("hidden", view !== "loading");
  $("view-login").classList.toggle("hidden", view !== "login");
  $("view-dash").classList.toggle("hidden", view !== "dash");

  // Disable auto-hide while the login form is active so the window stays
  // open even when focus briefly leaves.
  invoke("set_auto_hide", { enabled: view !== "login" });
}

/// Resize the popup height to match the rendered content.
async function fitHeight() {
  const el = document.getElementById("view-dash");
  if (!el) return;
  const h = Math.ceil(el.getBoundingClientRect().height);
  try {
    await invoke("fit_height", { height: h });
  } catch {
    // non-critical
  }
}

// ---- Render -----------------------------------------------------------------
function render(p: PackageData) {
  // API 返回的 used_monthly 不准确，用 used_week 代替
  p.used_monthly = p.used_week;
  // Plan name + status
  $("plan-name").textContent = p.plan_display_name;
  const statusEl = $("plan-status");
  statusEl.textContent = p.status === "active" ? "正常" : p.status;
  statusEl.className = "badge " + (p.status === "active" ? "badge-ok" : "badge-warn");

  // Quota progress
  const used = p.total_quota - p.remaining_quota;
  const quotaPct = p.total_quota > 0 ? (used / p.total_quota) * 100 : 0;
  $("quota-text").textContent = `${num(p.used_monthly)} / ${compact(p.total_quota)} ${p.billing_unit}`;
  $("quota-bar").style.width = `${Math.min(quotaPct, 100)}%`;

  // Weekly progress
  const weeklyGroup = $("weekly-group");
  if (p.weekly_limit && p.weekly_limit > 0) {
    weeklyGroup.classList.remove("hidden");
    const wkPct = (p.used_weekly / p.weekly_limit) * 100;
    $("weekly-text").textContent = `${num(p.used_weekly)} / ${compact(p.weekly_limit)} ${p.billing_unit}`;
    $("weekly-bar").style.width = `${Math.min(wkPct, 100)}%`;
  } else {
    weeklyGroup.classList.add("hidden");
  }

  // Metrics grid
  $("used-weekly").textContent = num(p.used_weekly);
  $("used-monthly").textContent = num(p.used_monthly);
  $("rpm-limit").textContent = String(p.rpm_total_limit);

  // Detail rows
  $("billing-unit").textContent = p.billing_unit === "requests" ? "请求数" : p.billing_unit;
  $("expire-at").textContent = formatDate(p.absolute_expire_at);
  $("reset-at").textContent = p.weekly_reset_at ? formatDate(p.weekly_reset_at) : "—";

  // Models tags
  const modelsEl = $("models-list");
  modelsEl.innerHTML = "";
  if (p.supported_models && p.supported_models.length > 0) {
    for (const m of p.supported_models) {
      const tag = document.createElement("span");
      tag.className = "tag";
      tag.textContent = m;
      modelsEl.appendChild(tag);
    }
  } else {
    modelsEl.textContent = "—";
  }

  $("updated-at").textContent = "更新于 " + time();
  fitHeight();

  // Update tray title with today's usage (plain number, no suffix)
  updateTrayTitle(p.total_quota - p.remaining_quota);
}

async function updateTrayTitle(used: number) {
  const title = `\u2004${used}`;
  try {
    await invoke("set_tray_title", { title });
  } catch {
    // ignore
  }
}

// ---- Data fetch -------------------------------------------------------------
let refreshing = false;
async function refresh() {
  if (refreshing) return;
  refreshing = true;
  $("refresh-btn")?.classList.add("spinning");
  try {
    const data = await invoke<PackageData>("fetch_package");
    render(data);
  } catch (e) {
    const msg = String(e);
    if (msg.includes("no master key") || msg.includes("401")) {
      show("login");
    } else {
      $("updated-at").textContent = "出错: " + msg.slice(0, 40);
    }
  } finally {
    refreshing = false;
    $("refresh-btn")?.classList.remove("spinning");
  }
}

// ---- Login: save master key -------------------------------------------------
async function handleSaveKey(ev: SubmitEvent) {
  ev.preventDefault();
  let key = ($("master-key") as HTMLInputElement).value.trim();
  // If user pasted the full Authorization header value, strip the prefix.
  if (key.startsWith("Bearer ")) {
    key = key.slice("Bearer ".length).trim();
  }
  const errEl = $("login-error");
  const btn = $("login-btn") as HTMLButtonElement;
  errEl.classList.add("hidden");
  btn.disabled = true;
  btn.textContent = "验证中…";
  try {
    // Save first
    await invoke("save_master_key", { key });
    // Test the key by fetching
    await invoke("fetch_package");
    show("dash");
    await refresh();
    startTimer();
  } catch (e) {
    // If save succeeded but fetch failed, clear the bad key
    try { await invoke("clear_master_key"); } catch { /* ignore */ }
    errEl.textContent = "验证失败：" + String(e);
    errEl.classList.remove("hidden");
  } finally {
    btn.disabled = false;
    btn.textContent = "保存并查看";
  }
}

// ---- Timer ------------------------------------------------------------------
let timer: number | undefined;
function startTimer() {
  stopTimer();
  const ms = Number(($("interval") as HTMLSelectElement).value);
  timer = window.setInterval(refresh, ms);
}
function stopTimer() {
  if (timer) {
    clearInterval(timer);
    timer = undefined;
  }
}

// ---- Boot -------------------------------------------------------------------
async function boot() {
  // Wire up events
  $("login-form").addEventListener("submit", handleSaveKey);
  $("refresh-btn").addEventListener("click", refresh);
  $("logout-btn").addEventListener("click", async () => {
    stopTimer();
    await invoke("clear_master_key");
    show("login");
  });
  document.querySelectorAll(".quit").forEach((btn) => {
    btn.addEventListener("click", () => invoke("quit_app"));
  });
  $("interval").addEventListener("change", () => {
    startTimer();
    refresh();
  });

  // Let the backend know when the mouse enters / leaves the popup so it
  // can suppress auto-hide while the user is interacting with the panel.
  const appEl = document.getElementById("app");
  if (appEl) {
    appEl.addEventListener("mouseenter", () => invoke("set_mouse_in_window", { inWindow: true }));
    appEl.addEventListener("mouseleave", () => invoke("set_mouse_in_window", { inWindow: false }));
  }

  // On Windows, WindowEvent::Focused is unreliable for transparent /
  // alwaysOnTop / skipTaskbar windows.  The webview blur event is a
  // more dependable signal that the user clicked elsewhere.
  window.addEventListener("blur", () => invoke("start_hide_timer_cmd"));

  // When the popup becomes visible, refresh immediately.
  try {
    const win = getCurrentWindow();
    win.onFocusChanged(({ payload: focused }) => {
      if (focused) refresh();
    });
  } catch {
    // non-critical
  }

  // Decide initial view
  let hasKey = false;
  try {
    hasKey = await invoke<boolean>("has_master_key");
  } catch {
    // show login form anyway
  }
  if (hasKey) {
    show("dash");
    await refresh();
    startTimer();
  } else {
    show("login");
  }
}

boot();
