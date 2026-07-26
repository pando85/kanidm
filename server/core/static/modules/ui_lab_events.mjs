const output = document.querySelector("[data-ui-lab-event-log]");
const clearButton = document.querySelector("[data-ui-lab-clear-events]");
const entries = [];
const MAX_ENTRIES = 12;

function render() {
    if (!output) return;

    output.innerHTML = "";
    if (entries.length === 0) {
        const empty = document.createElement("li");
        empty.className = "ui-lab-event-empty";
        empty.textContent = "No semantic events yet.";
        output.append(empty);
        return;
    }

    for (const entry of entries) {
        const item = document.createElement("li");
        const label = document.createElement("strong");
        const payload = document.createElement("code");
        label.textContent = entry.name;
        payload.textContent = JSON.stringify(entry.detail);
        item.append(label, payload);
        output.append(item);
    }
}

function record(name, detail) {
    entries.unshift({ name, detail });
    entries.splice(MAX_ENTRIES);
    render();
}

window.addEventListener("kubidm:guide-state", (event) => {
    record("kubidm:guide-state", event.detail);
});

for (const name of ["start", "submit", "cancelled", "error"]) {
    window.addEventListener(`kubidm:webauthn-${name}`, (event) => {
        record(`kubidm:webauthn-${name}`, event.detail || {});
    });
}

clearButton?.addEventListener("click", () => {
    entries.length = 0;
    render();
});

render();
