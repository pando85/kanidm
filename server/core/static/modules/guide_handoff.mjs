const AUTH_PENDING_KEY = "kubidm.guide.auth-pending";
const AUTH_PENDING_TTL_MS = 2 * 60 * 1000;

function storage() {
    try {
        return window.sessionStorage;
    } catch {
        return null;
    }
}

export function markAuthenticationAttempt() {
    storage()?.setItem(AUTH_PENDING_KEY, String(Date.now()));
}

export function clearAuthenticationAttempt() {
    storage()?.removeItem(AUTH_PENDING_KEY);
}

export function consumeConfirmedAuthenticationArrival(now = Date.now()) {
    const store = storage();
    if (!store) return false;

    const raw = store.getItem(AUTH_PENDING_KEY);
    store.removeItem(AUTH_PENDING_KEY);
    if (!raw) return false;

    const startedAt = Number(raw);
    if (!Number.isFinite(startedAt)) return false;

    const age = now - startedAt;
    return age >= 0 && age <= AUTH_PENDING_TTL_MS;
}
