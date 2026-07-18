import { access, readFile } from "node:fs/promises";
import path from "node:path";
import process from "node:process";

const root = process.cwd();
const read = (file) => readFile(path.join(root, file), "utf8");
const readJson = async (file) => JSON.parse(await read(file));

const rootPackage = await readJson("package.json");
const version = rootPackage.version;
const expected = new Map([
  [
    "apps/desktop/package.json",
    (await readJson("apps/desktop/package.json")).version,
  ],
  [
    "apps/desktop/src-tauri/tauri.conf.json",
    (await readJson("apps/desktop/src-tauri/tauri.conf.json")).version,
  ],
]);

const cargo = await read("Cargo.toml");
expected.set(
  "Cargo.toml",
  cargo.match(/\[workspace\.package\][\s\S]*?\nversion = "([^"]+)"/)?.[1],
);

const spec = await read("packaging/fedora/loofi-supportkit.spec");
expected.set(
  "packaging/fedora/loofi-supportkit.spec",
  spec.match(/^Version:\s*(\S+)/m)?.[1],
);

for (const [file, actual] of expected) {
  if (actual !== version) {
    throw new Error(
      `${file} declares ${actual ?? "no version"}; expected ${version}`,
    );
  }
}

const changelog = await read("CHANGELOG.md");
if (!changelog.includes(`## [${version}]`)) {
  throw new Error(`CHANGELOG.md has no ${version} release section`);
}

const releaseNotes = `docs/releases/${version}.md`;
await access(path.join(root, releaseNotes));

const requestedTag = process.argv[2];
if (requestedTag && requestedTag !== `v${version}`) {
  throw new Error(
    `tag ${requestedTag} does not match package version v${version}`,
  );
}

console.log(`Release metadata is consistent for v${version}.`);
