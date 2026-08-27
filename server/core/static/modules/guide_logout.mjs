import { MascotState, Severity } from "./guide_contract.mjs";

const modal = document.querySelector("#signoutModal");
let previous = null;

function semanticNode(scene) {
    return scene?.querySelector("[data-guide-state]") || scene;
}

function setGoodbye() {
    const scene = document.querySelector("[data-guide-scene]");
    const node = semanticNode(scene);
    if (!scene || !node) return;

    previous = {
        node,
        state: node.dataset.guideState,
        action: node.dataset.guideAction,
        severity: node.dataset.guideSeverity,
    };

    node.dataset.guideState = MascotState.GOODBYE;
    node.dataset.guideAction = "logout_confirm";
    node.dataset.guideSeverity = Severity.NEUTRAL;
}

function restore() {
    if (!previous?.node?.isConnected) {
        previous = null;
        return;
    }

    const { node, state, action, severity } = previous;
    if (state) node.dataset.guideState = state;
    else delete node.dataset.guideState;
    if (action) node.dataset.guideAction = action;
    else delete node.dataset.guideAction;
    if (severity) node.dataset.guideSeverity = severity;
    else delete node.dataset.guideSeverity;
    previous = null;
}

modal?.addEventListener("shown.bs.modal", setGoodbye);
modal?.addEventListener("hidden.bs.modal", restore);
