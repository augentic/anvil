// RFC-25 adapter manifest validation:
//   - every adapters/sources/<name>/adapter.yaml validates against source.schema.json,
//   - every adapters/targets/<name>/adapter.yaml validates against target.schema.json.

import {
  Ajv2020,
  fail,
  formatSchemaError,
  join,
  parseYaml,
  relative,
  REPO_ROOT,
  resolveSpecifyCliSchemasDir,
  SOURCES_DIR,
  TARGETS_DIR,
  walk,
} from "./_shared.ts";

type Validator = ((data: unknown) => boolean) & {
  errors?: import("./_shared.ts").AjvValidationError[];
};

async function loadValidator(schemaFile: string): Promise<Validator> {
  const ajv = new Ajv2020({ allErrors: true });
  const schema = JSON.parse(
    await Deno.readTextFile(join(resolveSpecifyCliSchemasDir(), schemaFile)),
  );
  return ajv.compile(schema);
}

async function validateManifest(
  path: string,
  validate: Validator,
): Promise<void> {
  const rel = relative(REPO_ROOT, path);
  const data = parseYaml(await Deno.readTextFile(path));
  if (!validate(data)) {
    for (const err of validate.errors ?? []) {
      fail(`Adapter validation failed: ${rel} — ${formatSchemaError(err)}`);
    }
  }
}

export async function validateAdapterYaml(): Promise<void> {
  const validateSource = await loadValidator("source.schema.json");
  const validateTarget = await loadValidator("target.schema.json");

  for await (
    const entry of walk(SOURCES_DIR, {
      maxDepth: 2,
      includeDirs: false,
      match: [/adapter\.yaml$/],
    })
  ) {
    await validateManifest(entry.path, validateSource);
  }

  for await (
    const entry of walk(TARGETS_DIR, {
      maxDepth: 2,
      includeDirs: false,
      match: [/adapter\.yaml$/],
    })
  ) {
    await validateManifest(entry.path, validateTarget);
  }
}
