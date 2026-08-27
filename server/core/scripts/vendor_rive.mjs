import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { cp, mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";

const PACKAGE = "@rive-app/canvas-lite";
const VERSION = "2.39.1";
const target = path.resolve("static", "rive");

function sha256(bytes) {
    return createHash("sha256").update(bytes).digest("hex");
}

function npmView(field) {
    return execFileSync("npm", ["view", `${PACKAGE}@${VERSION}`, field], {
        encoding: "utf8",
        stdio: ["ignore", "pipe", "inherit"],
    }).trim();
}

const temp = await mkdtemp(path.join(os.tmpdir(), "kubidm-rive-"));
try {
    const output = execFileSync("npm", ["pack", `${PACKAGE}@${VERSION}`, "--json"], {
        cwd: temp,
        encoding: "utf8",
        stdio: ["ignore", "pipe", "inherit"],
    });
    const pack = JSON.parse(output);
    const filename = pack?.[0]?.filename;
    if (!filename) throw new Error("npm pack did not return a tarball filename");

    execFileSync("tar", ["-xzf", filename], { cwd: temp, stdio: "inherit" });
    const packageRoot = path.join(temp, "package");
    const packageJson = JSON.parse(await readFile(path.join(packageRoot, "package.json"), "utf8"));
    if (packageJson.name !== PACKAGE || packageJson.version !== VERSION) {
        throw new Error(`Unexpected Rive package ${packageJson.name}@${packageJson.version}`);
    }

    await mkdir(target, { recursive: true });
    const files = ["rive.js", "rive.wasm"];
    const hashes = {};
    for (const file of files) {
        const source = path.join(packageRoot, file);
        const bytes = await readFile(source);
        hashes[file] = sha256(bytes);
        await cp(source, path.join(target, file));
    }

    let licenseBytes = null;
    let licenseSource = null;
    let sourceGitHead = npmView("gitHead");
    for (const candidate of ["LICENSE", "LICENSE.md", "LICENSE.txt"]) {
        try {
            licenseBytes = await readFile(path.join(packageRoot, candidate));
            licenseSource = `npm:${candidate}`;
            break;
        } catch {
            // Fall back to the package's immutable upstream git revision below.
        }
    }

    if (!licenseBytes) {
        if (!/^[0-9a-f]{7,40}$/i.test(sourceGitHead)) {
            throw new Error(`${PACKAGE}@${VERSION} did not expose a usable gitHead for license retrieval`);
        }
        const licenseUrl = `https://raw.githubusercontent.com/rive-app/rive-wasm/${sourceGitHead}/LICENSE`;
        const licensePath = path.join(temp, "UPSTREAM_LICENSE");
        execFileSync("curl", ["--fail", "--location", "--silent", "--show-error", "--output", licensePath, licenseUrl]);
        licenseBytes = await readFile(licensePath);
        licenseSource = licenseUrl;
    }

    await writeFile(path.join(target, "LICENSE"), licenseBytes);
    hashes.LICENSE = sha256(licenseBytes);

    const metadata = {
        package: PACKAGE,
        version: VERSION,
        sourceGitHead,
        license: packageJson.license || "UNKNOWN",
        licenseFile: "LICENSE",
        licenseSource,
        generatedBy: "server/core/scripts/vendor_rive.mjs",
        files: hashes,
    };
    await writeFile(path.join(target, "VERSION.json"), `${JSON.stringify(metadata, null, 2)}\n`);
    console.log(`Vendored ${PACKAGE}@${VERSION} into ${target}`);
} finally {
    await rm(temp, { recursive: true, force: true });
}
