// Generate a local dependency inventory without publishing workstation paths.
import { spawnSync } from "node:child_process";
import { createHash, randomUUID } from "node:crypto";
import { mkdir, readFile, readdir, writeFile } from "node:fs/promises";
import path from "node:path";

const output = path.resolve(process.argv[2] ?? `artifacts/dependencies/notices-${randomUUID()}`);
await mkdir(path.dirname(output), { recursive: true });
await mkdir(output, { recursive: false });

function run(command, args) {
  const result = spawnSync(command, args, { encoding: "utf8", shell: false, maxBuffer: 20 * 1024 * 1024, windowsHide: true });
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error(`${command} dependency inventory failed with code ${result.status}`);
  return JSON.parse(result.stdout);
}

const cargo = run("cargo", ["metadata", "--locked", "--offline", "--filter-platform", "x86_64-pc-windows-msvc", "--format-version", "1", "--manifest-path", "src-tauri/Cargo.toml"]);
const lock = JSON.parse(await readFile("package-lock.json", "utf8"));
const rows = [];
const licenseFile = /^(licen[cs]e|copying|copyright|notice)([.\-_ ]|$)/i;

async function collect(ecosystem, name, version, license, directory) {
  const id = `${ecosystem}:${name}@${version}`;
  const safeName = id.replace(/[^A-Za-z0-9._-]/g, "_");
  const texts = [];
  const entries = await readdir(directory, { withFileTypes: true });
  for (const entry of entries) {
    if (!entry.isFile() || !licenseFile.test(entry.name)) continue;
    const content = await readFile(path.join(directory, entry.name));
    const relative = `${safeName}/${entry.name}`;
    await mkdir(path.join(output, safeName), { recursive: true });
    await writeFile(path.join(output, relative), content, { flag: "wx" });
    texts.push({ path: relative, sha256: createHash("sha256").update(content).digest("hex") });
  }
  rows.push({ ecosystem, name, version, license: license ?? "NOASSERTION", license_files: texts });
}

for (const pkg of cargo.packages) {
  if (!pkg.source) continue;
  await collect("cargo", pkg.name, pkg.version, pkg.license, path.dirname(pkg.manifest_path));
}
for (const [location, pkg] of Object.entries(lock.packages)) {
  if (!location || pkg.dev) continue;
  const metadata = JSON.parse(await readFile(path.join(location, "package.json"), "utf8"));
  await collect("npm", metadata.name, pkg.version, pkg.license ?? metadata.license, location);
}
rows.sort((left, right) => `${left.ecosystem}:${left.name}`.localeCompare(`${right.ecosystem}:${right.name}`));
await writeFile(path.join(output, "dependencies.json"), `${JSON.stringify(rows, null, 2)}\n`, { flag: "wx" });
const lines = [
  "# Third-party dependency inventory", "",
  "Generated from locked npm runtime dependencies and the Cargo dependency graph for Windows x64.",
  "The Rust inventory includes build and test dependencies. This is a conservative inventory, not a claim that every package is linked into the binary.",
  "License files found in each installed package are included beside this file. Review the listed license expressions and any entries without license text before distribution.", "",
  "| Ecosystem | Package | Version | Declared license | License text files |",
  "| --- | --- | --- | --- | --- |",
  ...rows.map((pkg) => `| ${pkg.ecosystem} | ${pkg.name} | ${pkg.version} | ${pkg.license.replace(/\|/g, "\\|")} | ${pkg.license_files.length} |`),
];
await writeFile(path.join(output, "THIRD-PARTY-NOTICES.md"), `${lines.join("\n")}\n`, { flag: "wx" });
const spdx = {
  spdxVersion: "SPDX-2.3", dataLicense: "CC0-1.0", SPDXID: "SPDXRef-DOCUMENT",
  name: "SwiftFetch Windows dependency inventory",
  documentNamespace: `https://spdx.org/spdxdocs/swiftfetch-${randomUUID()}`,
  creationInfo: { created: new Date().toISOString().replace(/\.\d{3}Z$/, "Z"), creators: ["Tool: SwiftFetch dependency-notices"] },
  packages: rows.map((pkg, index) => ({
    SPDXID: `SPDXRef-Package-${index}`, name: pkg.name, versionInfo: pkg.version,
    downloadLocation: "NOASSERTION", filesAnalyzed: false,
    licenseConcluded: "NOASSERTION", licenseDeclared: pkg.license,
    copyrightText: "NOASSERTION",
    externalRefs: [{ referenceCategory: "PACKAGE-MANAGER", referenceType: "purl", referenceLocator: `pkg:${pkg.ecosystem}/${pkg.name.replace(/^@/, "%40")}@${pkg.version}` }],
  })),
};
await writeFile(path.join(output, "sbom.spdx.json"), `${JSON.stringify(spdx, null, 2)}\n`, { flag: "wx" });
console.log(`Dependency inventory: ${rows.length} packages; license text missing for ${rows.filter((pkg) => pkg.license_files.length === 0).length}; ${output}`);
