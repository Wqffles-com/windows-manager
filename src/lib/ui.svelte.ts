// App-wide UI state: mode, admin status, toasts and confirm dialogs.

type Toast = { msg: string; kind: "ok" | "error"; action?: { label: string; run: () => void } };
type Confirm = { title: string; msg: string; ok: string; resolve: (yes: boolean) => void };

const saved = (() => {
  try {
    return localStorage.getItem("mode");
  } catch {
    return null;
  }
})();

export const ui = $state({
  mode: (saved === "advanced" ? "advanced" : "normal") as "normal" | "advanced",
  elevated: false,
  toast: null as Toast | null,
  confirm: null as Confirm | null,
});

export function setMode(mode: "normal" | "advanced") {
  ui.mode = mode;
  try {
    localStorage.setItem("mode", mode);
  } catch {}
}

let timer: ReturnType<typeof setTimeout>;
export function toast(msg: string, kind: Toast["kind"] = "ok", action?: Toast["action"]) {
  ui.toast = { msg, kind, action };
  clearTimeout(timer);
  timer = setTimeout(() => (ui.toast = null), action ? 8000 : 4000);
}

export const fail = (e: unknown) => toast(String(e), "error");

export function ask(title: string, msg: string, ok = "Delete"): Promise<boolean> {
  return new Promise((resolve) => (ui.confirm = { title, msg, ok, resolve }));
}
