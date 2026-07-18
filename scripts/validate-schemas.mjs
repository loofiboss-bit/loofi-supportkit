import { readFile, readdir } from 'node:fs/promises';
import path from 'node:path';
import process from 'node:process';
import Ajv2020 from 'ajv/dist/2020.js';
import addFormats from 'ajv-formats';

const root = process.cwd();
const readJson = async (file) => JSON.parse(await readFile(path.join(root, file), 'utf8'));
const ajv = new Ajv2020({ allErrors: true, strict: true });
addFormats(ajv);

for (const name of ['evidence.schema.json', 'finding.schema.json']) {
  ajv.addSchema(await readJson(`schemas/${name}`));
}
const ruleSchema = await readJson('schemas/rule.schema.json');
ajv.addSchema(await readJson('schemas/support-case.schema.json'));
const validateRule = ajv.compile(ruleSchema);

for (const platform of ['common', 'fedora', 'windows']) {
  const directory = path.join(root, 'rules', platform);
  for (const name of (await readdir(directory)).sort()) {
    const document = JSON.parse(await readFile(path.join(directory, name), 'utf8'));
    if (!validateRule(document)) {
      throw new Error(`${platform}/${name}: ${ajv.errorsText(validateRule.errors)}`);
    }
  }
}

const evidenceSchema = ajv.getSchema('https://loofi.dev/supportkit/schemas/evidence.schema.json');
for (const evidence of await readJson('fixtures/fedora-general-evidence.json')) {
  if (!evidenceSchema(evidence)) throw new Error(ajv.errorsText(evidenceSchema.errors));
}

console.log('Schemas, declarative rules, and fixture evidence are valid.');

