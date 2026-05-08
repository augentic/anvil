/** Append text to a log path. */
export async function appendLog(path: string, text: string): Promise<void> {
  await Deno.writeTextFile(path, text, { append: true });
}
