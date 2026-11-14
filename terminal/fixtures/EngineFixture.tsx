import { mount, signal, Text } from "nsh";
import { Button, Terminal, type TerminalRef } from "nsh/base";
import { TerminalEngine } from "termlevel/native";

// Exercises the native provider and NSH Terminal retained by the preview, without
// the preview command interpreter. No subprocesses or network are involved.
function EngineFixture() {
  let terminal: TerminalRef | undefined;
  let started = false;
  const input = signal("");
  const title = signal("");
  return <TerminalEngine class="flex-1">
    <Terminal testID="engine-terminal" ref={(ref) => { terminal = ref; }} class="flex-1"
      onData={({ data }) => input.set(JSON.stringify(data))}
      onTitleChange={({ title: value }) => title.set(value)}
      onResize={() => {
        if (started) return;
        started = true;
        // REP is supported by Ghostty but absent from NSH's default VT.
        // This makes silently falling back to the old emulator observable.
        terminal?.write("Q\x1b[4b\r\n\x1b[38;2;12;34;56m中文é\x1b[0m");
      }} />
    <Text>{input()}</Text>
    <Text>{title()}</Text>
    <Button testID="engine-alternate" onPress={() => terminal?.write("\x1b[?1049h\x1b[2J\x1b[Halternate")}><Text>Alternate</Text></Button>
    <Button testID="engine-primary" onPress={() => terminal?.write("\x1b[?1049l")}><Text>Primary</Text></Button>
    <Button testID="engine-modes" onPress={() => {
      terminal?.write("\x1b[?1h\x1b[?2004h");
      terminal?.focus();
    }}><Text>Input modes</Text></Button>
    <Button testID="engine-query" onPress={() => terminal?.write("\x1b]0;Ghostty test\x07\x1b[6n")}><Text>Query</Text></Button>
  </TerminalEngine>;
}

mount(EngineFixture);
