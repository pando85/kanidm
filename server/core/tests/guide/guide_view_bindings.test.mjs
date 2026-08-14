import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const viewsMod = await readFile(new URL("../../src/https/views/mod.rs", import.meta.url), "utf8");

function requireFragment(fragment, description) {
    assert.ok(viewsMod.includes(fragment), `${description}: expected ${JSON.stringify(fragment)}`);
}

test("guide semantics and debug UI Lab stay exported and routed", () => {
    requireFragment("pub(crate) mod guide;", "guide helpers must remain available to Askama templates");
    requireFragment('#[cfg(debug_assertions)]\nmod ui_lab;', "UI Lab module must stay debug-only");
    requireFragment('std::env::var_os("KUBIDM_UI_LAB")', "UI Lab route must require explicit opt-in");
    requireFragment('route("/_lab", get(ui_lab::view_lab_get))', "UI Lab route must remain available to browser CI");
});
