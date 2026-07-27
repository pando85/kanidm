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

    const metadata = {
        package: PACKAGE,
        version: VERSION,
        license: packageJson.license || "MIT",
        generatedBy: "server/core/scripts/vendor_rive.mjs",
        files: hashes,
    };
    await writeFile(path.join(target, "VERSION.json"), `${JSON.stringify(metadata, null, 2)}\n`);
    console.log(`Vendored ${PACKAGE}@${VERSION} into ${target}`);
} finally {
    await rm(temp, { recursive: true, force: true });
}
