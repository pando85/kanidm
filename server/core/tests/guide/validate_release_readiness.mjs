import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { readFile, stat } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const [manifestPath, reviewPath, humanApprovalPath] = process.argv.slice(2);
if (!manifestPath || !reviewPath || !humanApprovalPath) {
    console.error(
        "usage: node tests/guide/validate_release_readiness.mjs <manifest.json> <visual-review.json> <human-approval.json>",
    );
    process.exit(2);
}

const coreRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const rivPath = path.join(coreRoot, "static", "img", "guide", "kubidm-guide.riv");
const visualValidator = path.join(coreRoot, "tests", "guide", "validate_visual_review.mjs");
const errors = [];

async function json(filename, label) {
    try {
        return JSON.parse(await readFile(filename, "utf8"));
    } catch (error) {
        errors.push(`${label} could not be read as JSON: ${error.message}`);
        return null;
    }
}

async function sha256(filename) {
    return createHash("sha256").update(await readFile(filename)).digest("hex");
}

let rivSha256 = null;
try {
    const rivStat = await stat(rivPath);
    if (!rivStat.isFile() || rivStat.size === 0) errors.push("production kubidm-guide.riv is empty");
    else rivSha256 = await sha256(rivPath);
} catch {
    errors.push("production static/img/guide/kubidm-guide.riv is missing");
}

const manifest = await json(manifestPath, "evidence manifest");
if (manifest) {
    if (manifest.mode !== "real") errors.push(`evidence mode must be real, got ${manifest.mode}`);
    if (manifest.fullMatrix !== true) errors.push("evidence must be generated with KUBIDM_GUIDE_FULL_MATRIX=1");
    if (!manifest.commit || typeof manifest.commit !== "string") errors.push("evidence commit is missing");
    if (!manifest.rivSha256) errors.push("evidence does not record a .riv SHA-256");
    if (rivSha256 && manifest.rivSha256 !== rivSha256) {
        errors.push(`evidence .riv SHA-256 ${manifest.rivSha256} does not match current asset ${rivSha256}`);
    }
    if (!Array.isArray(manifest.externalRequests) || manifest.externalRequests.length !== 0) {
        errors.push("production evidence contains external network requests");
    }

    const captures = Array.isArray(manifest.captures) ? manifest.captures : [];
    const expectedViewports = ["desktop", "tablet", "mobile"];
    const expectedThemes = ["light", "dark"];
    const expectedMotions = ["full", "reduced", "static"];
    for (const viewport of expectedViewports) {
        for (const theme of expectedThemes) {
            for (const motion of expectedMotions) {
                if (
                    !captures.some(
                        (capture) => capture.viewport === viewport && capture.theme === theme && capture.motion === motion,
                    )
                ) {
                    errors.push(`evidence is missing ${viewport}/${theme}/${motion}`);
                }
            }
        }
    }
    if (captures.some((capture) => capture.motion === "full" && capture.diagnostic?.fallbackActive)) {
        errors.push("full-motion evidence contains a static fallback");
    }
}

try {
    execFileSync(process.execPath, [visualValidator, reviewPath], { stdio: "inherit" });
} catch {
    errors.push("independent visual review did not pass production thresholds");
}

const human = await json(humanApprovalPath, "human approval");
if (human) {
    if (!rivSha256 || human.riv_sha256 !== rivSha256) {
        errors.push("human approval must record the current .riv SHA-256");
    }
    if (manifest?.commit && human.evidence_commit !== manifest.commit) {
        errors.push("human approval must reference the evidence commit");
    }

    for (const gate of ["silhouette", "travel_gait"]) {
        const approval = human[gate];
        if (!approval || approval.approved !== true) {
            errors.push(`human ${gate} approval is required`);
            continue;
        }
        if (typeof approval.reviewer !== "string" || approval.reviewer.trim().length === 0) {
            errors.push(`human ${gate} approval must name a reviewer`);
        }
        if (typeof approval.reviewed_at !== "string" || Number.isNaN(Date.parse(approval.reviewed_at))) {
            errors.push(`human ${gate} approval must include an ISO-compatible reviewed_at timestamp`);
        }
    }
}

if (errors.length > 0) {
    console.error("Kubidm production Rive release gate failed:");
    for (const error of errors) console.error(`- ${error}`);
    process.exit(1);
}

console.log(`Kubidm production Rive release gate passed for ${rivSha256}`);
