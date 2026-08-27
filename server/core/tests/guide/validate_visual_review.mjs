import { readFile } from "node:fs/promises";

const reviewPath = process.argv[2];
if (!reviewPath) {
    console.error("usage: node tests/guide/validate_visual_review.mjs <review.json>");
    process.exit(2);
}

const expectedScores = [
    "silhouette_fidelity",
    "proportion_fidelity",
    "band_badge_fidelity",
    "face_fidelity",
    "claw_role_readability",
    "pose_semantic_readability",
    "motion_smoothness",
    "lack_of_clipping_deformation",
    "accessibility_appropriateness",
    "product_state_consistency",
];

const review = JSON.parse(await readFile(reviewPath, "utf8"));
const errors = [];

if (typeof review.reviewer !== "string" || review.reviewer.trim().length === 0) {
    errors.push("reviewer must name the independent reviewer or review process");
}
if (typeof review.reviewed_at !== "string" || Number.isNaN(Date.parse(review.reviewed_at))) {
    errors.push("reviewed_at must be an ISO-compatible timestamp");
}
if (typeof review.evidence_commit !== "string" || review.evidence_commit.trim().length < 7) {
    errors.push("evidence_commit must identify the reviewed evidence commit");
}
if (typeof review.riv_sha256 !== "string" || !/^[0-9a-f]{64}$/i.test(review.riv_sha256)) {
    errors.push("riv_sha256 must be a 64-character SHA-256 hex digest");
}
if (typeof review.pass !== "boolean") errors.push("pass must be boolean");
if (!review.scores || typeof review.scores !== "object" || Array.isArray(review.scores)) {
    errors.push("scores must be an object");
} else {
    const actualScoreNames = Object.keys(review.scores).sort();
    const expectedScoreNames = [...expectedScores].sort();
    if (JSON.stringify(actualScoreNames) !== JSON.stringify(expectedScoreNames)) {
        errors.push(`scores must contain exactly: ${expectedScores.join(", ")}`);
    }
    for (const name of expectedScores) {
        const value = review.scores[name];
        if (!Number.isInteger(value) || value < 0 || value > 5) {
            errors.push(`${name} must be an integer from 0 to 5`);
        } else if (value < 4) {
            errors.push(`${name}=${value} fails the minimum production score of 4`);
        }
    }
}

for (const name of ["blocking_defects", "non_blocking_defects", "recommended_changes"]) {
    if (!Array.isArray(review[name]) || review[name].some((item) => typeof item !== "string" || item.length === 0)) {
        errors.push(`${name} must be an array of non-empty strings`);
    }
}

if (Array.isArray(review.blocking_defects) && review.blocking_defects.length > 0) {
    errors.push(`blocking defects remain: ${review.blocking_defects.join("; ")}`);
}
if (review.pass !== true) errors.push("review pass must be true");

if (errors.length > 0) {
    console.error("Kubidm Rive visual review failed:");
    for (const error of errors) console.error(`- ${error}`);
    process.exit(1);
}

console.log("Kubidm Rive visual review passed production thresholds");
