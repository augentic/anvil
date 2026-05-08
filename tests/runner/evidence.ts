/** Append text to a log path. Backends and the runner share these helpers. */
export async function appendLog(path: string, text: string): Promise<void> {
  await Deno.writeTextFile(path, text, { append: true });
}
