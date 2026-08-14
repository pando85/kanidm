import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readdir, readFile } from "node:fs/promises";
import test from "node:test";
import { brotliCompressSync, constants as zlibConstants } from "node:zlib";

const staticRoot = new URL("../../static/", import.meta.url);
const guideRoot = new URL("../../static/img/guide/", import.meta.url);
const contract = JSON.parse(await readFile(new URL("guide_rive_contract.json", staticRoot), "utf8"));
const runtimeVersion = JSON.parse(await readFile(new URL("rive/VERSION.json", staticRoot), "utf8"));

function sha256(bytes) {
    return createHash("sha256").update(bytes).digest("hex");
}

test("vendored Rive runtime and license bytes match committed SHA-256 metadata", async () => {
    for (const filename of ["rive.js", "rive.wasm", "LICENSE"]) {
        const bytes = await readFile(new URL(`rive/${filename}`, staticRoot));
        assert.equal(sha256(bytes), runtimeVersion.files[filename], `${filename} hash drift`);
    }
    assert.equal(runtimeVersion.license, "MIT");
    assert.equal(runtimeVersion.licenseFile, "LICENSE");
    assert.match(runtimeVersion.sourceGitHead, /^[0-9a-f]{40}$/i);
    assert.match(runtimeVersion.licenseSource, new RegExp(runtimeVersion.sourceGitHead));
});

test("canonical static fallback pack is complete WebP and contains no crab SVG poses", async () => {
    const expected = [
        "crab-idle.webp",
        "crab-welcome.webp",
        "crab-guide.webp",
        "crab-protect.webp",
        "crab-working.webp",
        "crab-success.webp",
        "crab-warning.webp",
        "crab-goodbye.webp",
    ];
    const files = await readdir(guideRoot);
    assert.deepEqual(files.filter((name) => /^crab-.*\.webp$/.test(name)).sort(), expected.sort());
    assert.deepEqual(
        files.filter((name) => /^crab-.*\.svg$/.test(name)),
        [],
    );

    for (const filename of expected) {
        const bytes = await readFile(new URL(filename, guideRoot));
        assert.equal(bytes.subarray(0, 4).toString("ascii"), "RIFF", `${filename} is not RIFF WebP`);
        assert.equal(bytes.subarray(8, 12).toString("ascii"), "WEBP", `${filename} is not WebP`);
    }
});

test("CSS contains no full-motion character keyframe implementation", async () => {
    const css = await readFile(new URL("guide_motion.css", staticRoot), "utf8");
    assert.equal(css.includes("@keyframes"), false);
    assert.equal(css.includes("guide-travel-arrival"), false);
    assert.equal(css.includes("guide-success"), false);
    assert.match(css, /Internal character animation belongs exclusively to the Rive state machine/);
});

test("real Rive asset obeys production size budget when present", async (t) => {
    const assetUrl = new URL("kubidm-guide.riv", guideRoot);
    let bytes;
    try {
        bytes = await readFile(assetUrl);
    } catch (error) {
        if (process.env.KUBIDM_EXPECT_REAL_RIVE === "1") throw error;
        t.skip("kubidm-guide.riv is authored in the external Rive environment");
        return;
    }

    assert.ok(bytes.length > 0, "kubidm-guide.riv must not be empty");
    const compressed = brotliCompressSync(bytes, {
        params: {
            [zlibConstants.BROTLI_PARAM_QUALITY]: 11,
        },
    });
    assert.ok(
        compressed.length <= 250 * 1024,
        `kubidm-guide.riv Brotli size ${compressed.length} exceeds 250 KiB production target`,
    );
    assert.equal(contract.asset, "/pkg/img/guide/kubidm-guide.riv");
});
