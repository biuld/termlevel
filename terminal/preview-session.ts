/** A deterministic UI-preview transport. It never starts an OS process. */
export class PreviewSession {
  private line = "";
  private history: string[] = [];
  private historyIndex = 0;

  constructor(
    private write: (text: string) => void,
    private cwd: string,
    private colors: () => { brand: string; prompt: string; hint: string },
  ) {}

  start() {
    this.write(`\x1b[1m${this.colors().brand}termlevel\x1b[0m\r\n`);
    this.write(`${this.colors().hint}A little room for everything you're working on.\x1b[0m\r\n\r\n`);
    this.write("Interactive UI preview. No shell is connected yet.\r\n");
    this.write("Try \x1b[36mhelp\x1b[0m, \x1b[36mcolors\x1b[0m or \x1b[36mecho hello\x1b[0m.\r\n\r\n");
    this.prompt();
  }

  private prompt() {
    this.write(`${this.colors().prompt}❯\x1b[0m `);
  }

  private replaceLine(next: string) {
    this.write(`\r\x1b[2K${this.colors().prompt}❯\x1b[0m ${next}`);
    this.line = next;
  }

  /** Returns a process exit code when the preview shell leaves. Ctrl+D on an empty line is 0. */
  input(data: string): number | void {
    if (data === "\x1b[A" || data === "\x1b[B") {
      this.historyIndex = Math.max(0, Math.min(this.history.length,
        this.historyIndex + (data === "\x1b[A" ? -1 : 1)));
      this.replaceLine(this.history[this.historyIndex] ?? "");
      return;
    }
    // The preview deliberately supports a small command set, not shell syntax.
    if (data.startsWith("\x1b") && !data.startsWith("\x1b[200~")) return;
    const input = data.replace(/\x1b\[20[01]~/g, "");
    for (const ch of input) {
      if (ch === "\r" || ch === "\n") {
        this.write("\r\n");
        const command = this.line;
        this.line = "";
        if (command.trim()) this.history.push(command);
        this.historyIndex = this.history.length;
        const code = this.run(command.trim());
        if (typeof code === "number") return code;
        this.prompt();
      } else if (ch === "\x7f" || ch === "\b") {
        if (this.line) this.replaceLine(Array.from(this.line).slice(0, -1).join(""));
      } else if (ch === "\x03") {
        this.line = "";
        this.write("^C\r\n");
        this.prompt();
      } else if (ch === "\x0c") {
        this.write("\x1b[2J\x1b[H");
        this.prompt();
        this.write(this.line);
      } else if (ch === "\x04") {
        if (!this.line) return 0;
      } else if (ch >= " ") {
        this.line += ch;
        this.write(ch);
      }
    }
  }

  private run(command: string): number | void {
    const [name, ...args] = command.split(/\s+/);
    switch (name) {
      case "": return;
      case "help":
        this.write("\x1b[1mPreview commands\x1b[0m\r\n");
        this.write("  help          Show this list\r\n  echo <text>   Print some text\r\n");
        this.write("  pwd           Show this tab's demo directory\r\n  colors        ANSI and truecolor samples\r\n");
        this.write("  clear         Clear the screen\r\n\r\n");
        this.write("Use ↑/↓ for history, Ctrl+C to cancel, Ctrl+D or exit to close, Ctrl+L to clear.\r\n");
        this.write("Other commands need the real shell planned for M2.\r\n");
        return;
      case "echo": this.write(args.join(" ") + "\r\n"); return;
      case "pwd": this.write(this.cwd + "\r\n"); return;
      case "clear": this.write("\x1b[2J\x1b[H"); return;
      case "exit": {
        const code = args.length === 0 ? 0 : Number(args[0]);
        if (!Number.isInteger(code) || code < 0 || code > 255) {
          this.write("exit: code must be an integer from 0 to 255\r\n");
          return;
        }
        return code;
      }
      case "colors":
        this.write("\r\n");
        for (let i = 0; i < 16; i++) this.write(`\x1b[48;5;${i}m   \x1b[0m${i === 7 || i === 15 ? "\r\n" : ""}`);
        this.write("\r\n");
        for (let i = 0; i < 32; i++) this.write(`\x1b[48;2;${197 - i * 3};${120 + i * 3};255m \x1b[0m`);
        this.write("\r\n\r\n");
        return;
      default:
        this.write(`\x1b[33mPreview: '${name}' needs a real shell. Type help for available commands.\x1b[0m\r\n`);
    }
  }
}
