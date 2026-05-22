// Schema validators sourced from the sibling `specify-cli/schemas/`
// directory. Used by the acceptance harness to schema-validate Evidence,
// candidate, plan, and target manifests without depending on the
// `specify` binary being on PATH.

import Ajv2020Module from "npm:ajv@8/dist/2020.js";
import { join, resolve } from "jsr:@std/path@1";
import { REPO_ROOT } from "./fixtures.ts";

type AjvValidator = ((data: unknown) => boolean) & {
  errors?: Array<{ instancePath?: string; message?: string }>;
};

const Ajv2020 = Ajv2020Module as unknown as {
  new (opts: { allErrors?: boolean }): {
    compile(schema: unknown): AjvValidator;
  };
};

function schemasDir(): string {
  const override = (() => {
    try {
      return Deno.env.get("SPECIFY_CLI_DIR");
    } catch {
      return undefined;
    }
  })();
  return join(resolve(REPO_ROOT, override ?? "../specify-cli"), "schemas");
}

const cache = new Map<string, AjvValidator>();

export async function loadValidator(file: string): Promise<AjvValidator | null> {
  if (cache.has(file)) return cache.get(file) ?? null;
  const path = join(schemasDir(), file);
  let txt: string;
  try {
    txt = await Deno.readTextFile(path);
  } catch {
    cache.set(file, null as unknown as AjvValidator);
    return null;
  }
  const ajv = new Ajv2020({ allErrors: true });
  const validator = ajv.compile(JSON.parse(txt));
  cache.set(file, validator);
  return validator;
}

export async function validateOrThrow(
  file: string,
  data: unknown,
  context: string,
): Promise<void> {
  const validate = await loadValidator(file);
  if (!validate) {
    // Sibling schemas not available; soft-skip with a console note.
    console.log(
      `  note: ${file} not found under SPECIFY_CLI_DIR; skipping schema check for ${context}`,
    );
    return;
  }
  if (!validate(data)) {
    const errs = (validate.errors ?? [])
      .map((e) => `${e.instancePath || "/"} ${e.message ?? "schema violation"}`)
      .join("; ");
    throw new Error(`${context} fails ${file}: ${errs}`);
  }
}
