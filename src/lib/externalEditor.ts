import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

export class ExternalEditorError extends Error {
  constructor(
    message: string,
    public readonly code: "missing-editor" | "launch-failed" | "editor-failed",
  ) {
    super(message);
    this.name = "ExternalEditorError";
  }
}

export async function editTextInExternalEditor(
  initialValue: string,
  options: { extension?: string; onResume?: () => void } = {},
): Promise<string> {
  const editor = process.env.VISUAL || process.env.EDITOR;
  if (!editor?.trim()) {
    throw new ExternalEditorError("Set $VISUAL or $EDITOR to use external editing", "missing-editor");
  }

  const dir = await mkdtemp(join(tmpdir(), "aion-editor-"));
  const filePath = join(dir, `field${options.extension ?? ".txt"}`);
  const stdin = process.stdin as typeof process.stdin & {
    isRaw?: boolean;
    setRawMode?: (mode: boolean) => void;
  };
  const wasRaw = !!stdin.isRaw;

  try {
    await writeFile(filePath, initialValue, "utf8");

    suspendTerminal(stdin);

    const proc = Bun.spawn(["sh", "-c", `${editor} "$1"`, "aion-editor", filePath], {
      stdin: "inherit",
      stdout: "inherit",
      stderr: "inherit",
    });
    const exitCode = await proc.exited;

    if (exitCode !== 0) {
      throw new ExternalEditorError(`Editor exited with code ${exitCode}`, "editor-failed");
    }

    return await readFile(filePath, "utf8");
  } catch (err) {
    if (err instanceof ExternalEditorError) {
      throw err;
    }
    throw new ExternalEditorError(
      err instanceof Error ? err.message : String(err),
      "launch-failed",
    );
  } finally {
    resumeTerminal(stdin, wasRaw);
    options.onResume?.();
    await rm(dir, { recursive: true, force: true });
  }
}

function suspendTerminal(stdin: NodeJS.ReadStream & { setRawMode?: (mode: boolean) => void }) {
  process.stdout.write("\x1B[0m\x1B]112\x07\x1B[<u\x1B[?25h\x1B[?1049l");

  if (stdin.isTTY && stdin.setRawMode) {
    stdin.setRawMode(false);
    stdin.pause();
  }
}

function resumeTerminal(
  stdin: NodeJS.ReadStream & { setRawMode?: (mode: boolean) => void },
  restoreRawMode: boolean,
) {
  if (stdin.isTTY && stdin.setRawMode && restoreRawMode) {
    stdin.setRawMode(true);
    stdin.resume();
    stdin.setEncoding("utf-8");
  }

  process.stdout.write("\x1B[?1049h\x1B[>1u\x1B[?25l\x1B[2J\x1B[H");
}
