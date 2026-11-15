import { keymap, Scroll, setColorScheme, signal, Text, token, View, type Signal } from "nsh";
import { Button } from "nsh/base";
import { AppDialog } from "./controls";
import { applyFonts, interfaceFont, interfaceFonts, monoFont, monoFonts } from "./fonts";
import { Type } from "./type";

const pages = [
  { id: "appearance", label: "Appearance" },
  { id: "terminal", label: "Terminal" },
  { id: "interface", label: "Interface" },
  { id: "shortcuts", label: "Shortcuts" },
  { id: "about", label: "About" },
] as const;

type SettingsPage = typeof pages[number]["id"];

const shortcuts = [
  { label: "New terminal", keys: "mod+t" },
  { label: "Close terminal", keys: "mod+w" },
  { label: "Split right", keys: "mod+d" },
  { label: "Split down", keys: "mod+shift+d" },
  { label: "Focus pane (use any arrow)", keys: "mod+alt+right" },
  { label: "Swap pane (use any arrow)", keys: "mod+alt+shift+right" },
  { label: "Maximize / restore pane", keys: "mod+shift+enter" },
  { label: "New group", keys: "mod+shift+g" },
  { label: "Find terminal", keys: "mod+k" },
  { label: "Find in terminal", keys: "mod+f" },
  { label: "Toggle sidebar", keys: "mod+b" },
  { label: "Next terminal", keys: "ctrl+tab" },
  { label: "Previous terminal", keys: "ctrl+shift+tab" },
  { label: "Switch to terminal 1", keys: "mod+1" },
  { label: "Settings", keys: "mod+," },
] as const;

/** Settings uses the window floor, with the section list beside one inset pane. */
export function SettingsDialog(props: {
  open: boolean;
  dismiss: () => void;
  appearance: Signal<"system" | "light" | "dark">;
  fontSize: Signal<number>;
  version: string;
  preview: boolean;
  scrollback: Signal<number>;
  clipboardRead: Signal<boolean>;
  scrollbackMin: number;
  scrollbackMax: number;
  scrollbackStep: number;
}) {
  const page = signal<SettingsPage>("appearance");
  const lineHeight = () => props.fontSize() + token("line-gap-terminal");
  return <AppDialog testID="settings-dialog" open={props.open} label="Settings" title="Settings"
    frame="w-[760px]" dismiss={props.dismiss} closeLabel="Close settings" padded={false}>
    <View class="h-[480px] flex-row px-2 pb-2">
      <View class="w-[148px] py-1" role="tablist" accessibilityLabel="Settings sections">
        <Each of={pages}>{(item) =>
          <Button
            testID={`settings-${item().id}`}
            role="tab"
            selected={page() === item().id}
            accessibilityLabel={`${item().label} settings`}
            onPress={() => page.set(item().id)}
            class="mx-1 h-(--height-tab) items-start justify-center rounded-(--radius-tab) px-2 hover:bg-tab-hover focus-visible:ring-2 ring-sidebar-ring"
            classList={{ "bg-tab-selected hover:bg-tab-selected": page() === item().id }}
          >
            <Text class="font-medium" style={{
              fontSize: token("text-body"),
              lineHeight: token("leading-body"),
              color: token(page() === item().id ? "tab-selected-foreground" : "sidebar-foreground"),
            }}>{item().label}</Text>
          </Button>
        }</Each>
      </View>
      <Scroll class="min-h-0 min-w-0 flex-1 rounded-(--radius-panel) bg-sidebar">
        <View class="gap-4 p-3">
          <If when={page() === "appearance"}>
            <View class="gap-2">
              <View class="flex-row rounded-(--radius-tab) bg-chrome p-0.5">
                <Each of={["system", "light", "dark"] as const}>{(mode) =>
                  <Button
                    testID={`appearance-${mode()}`}
                    accessibilityLabel={`${mode()} appearance`}
                    selected={props.appearance() === mode()}
                    onPress={() => { props.appearance.set(mode()); setColorScheme(mode()); }}
                    class="h-(--height-tab) flex-1 items-center justify-center rounded-(--radius-control) focus-visible:ring-2 ring-sidebar-ring"
                    classList={{ "bg-sidebar": props.appearance() === mode() }}
                  >
                    <Text class="font-medium" style={{
                      fontSize: token("text-label"),
                      lineHeight: token("leading-label"),
                      color: token(props.appearance() === mode() ? "sidebar-foreground" : "muted-foreground"),
                    }}>{mode() === "system" ? "System" : mode() === "light" ? "Light" : "Dark"}</Text>
                  </Button>
                }</Each>
              </View>
              <Type role="caption">Applies immediately. Open terminals stay mounted.</Type>
            </View>
          </If>
          <If when={page() === "terminal"}>
            <View class="gap-3">
              <View class="flex-row items-center gap-2">
                <View class="min-w-0 flex-1 gap-0.5">
                  <Type role="body">Font size</Type>
                  <Type role="caption">Size is the terminal only. The family is shared with the toolbar path, which stays at body size.</Type>
                </View>
                <Step label="Smaller terminal font" onPress={() => props.fontSize.update((n) => Math.max(10, n - 1))}>−</Step>
                <Type role="mono" class="w-12 text-center">{props.fontSize()} pt</Type>
                <Step label="Larger terminal font" onPress={() => props.fontSize.update((n) => Math.min(24, n + 1))}>+</Step>
              </View>
              <View class="flex-row items-center gap-2">
                <View class="min-w-0 flex-1 gap-0.5">
                  <Type role="body">Scrollback</Type>
                  <Type role="caption">Open terminals keep this many lines. Lowering it drops the oldest lines now. The setting is saved with the workspace.</Type>
                </View>
                <Step testID="scrollback-less" label="Fewer scrollback lines" onPress={() => props.scrollback.update((n) => Math.max(props.scrollbackMin, n - props.scrollbackStep))}>−</Step>
                <Type role="mono" class="w-16 text-center">{props.scrollback().toLocaleString("en-US")}</Type>
                <Step testID="scrollback-more" label="More scrollback lines" onPress={() => props.scrollback.update((n) => Math.min(props.scrollbackMax, n + props.scrollbackStep))}>+</Step>
              </View>
              <View class="flex-row items-center gap-2">
                <View class="min-w-0 flex-1 gap-0.5">
                  <Type role="body">Clipboard access</Type>
                  <Type role="caption">Terminal programs can copy text. Allow reading to support remote clipboard queries. Resets on restart.</Type>
                </View>
                <Button testID="clipboard-read" selected={props.clipboardRead()} onPress={() => props.clipboardRead.update((v) => !v)}>
                  <Type role="label">{props.clipboardRead() ? "Read and write" : "Copy only"}</Type>
                </Button>
              </View>
              <FontChoices fonts={monoFonts} selected={monoFont} testID="mono-font" onSelect={(id) => { monoFont.set(id); applyFonts(); }} />
              <View class="rounded-(--radius-tab) bg-chrome px-3 py-2">
                <Text style={{ fontFamily: monoFont(), fontSize: props.fontSize(), lineHeight: lineHeight(), color: token("terminal-prompt") }}>~/Projects/termlevel</Text>
                <Text style={{ fontFamily: monoFont(), fontSize: props.fontSize(), lineHeight: lineHeight(), color: token("terminal-foreground") }}>echo typography</Text>
              </View>
            </View>
          </If>
          <If when={page() === "interface"}>
            <View class="gap-3">
              <Type role="caption">System is the platform font. The toolbar path uses the terminal family at body size, {token("text-body")} pt.</Type>
              <FontChoices fonts={interfaceFonts} selected={interfaceFont} testID="ui-font" onSelect={(id) => { interfaceFont.set(id); applyFonts(); }} />
              <View class="gap-1 rounded-(--radius-tab) bg-chrome px-3 py-2">
                <Type role="title">Title {token("text-title")}</Type>
                <Type role="body">Body {token("text-body")} keeps names and settings readable.</Type>
                <Type role="body" family="mono" tone="muted-foreground">~/Projects/termlevel</Type>
                <Type role="label">Label {token("text-label")}</Type>
                <Type role="caption">Caption {token("text-caption")} for counts and hints.</Type>
              </View>
            </View>
          </If>
          <If when={page() === "shortcuts"}>
            <View>
              <Each of={shortcuts}>{(shortcut) => <View class="h-(--height-tab) flex-row items-center justify-between gap-3">
                <Type role="body">{shortcut().label}</Type>
                <Type role="mono">{keymap.label(shortcut().keys)}</Type>
              </View>}</Each>
            </View>
          </If>
          <If when={page() === "about"}>
            <View class="gap-1">
              <Type role="body">Termlevel {props.version}</Type>
              <Type role="caption">{props.preview ? "Interactive UI preview. No shell is connected. Workspace and settings are not saved in preview." : "Native terminal sessions powered by Ghostty. Closing a terminal ends its processes. The next launch restores the workspace and starts fresh shells in the saved directories."}</Type>
              <If when={!props.preview}>
                <Type role="caption">Settings live in ~/Library/Application Support/app.termlevel.desktop/storage.json.</Type>
              </If>
            </View>
          </If>
        </View>
      </Scroll>
    </View>
  </AppDialog>;
}

function Step(props: { label: string; testID?: string; onPress: () => void; children: string }) {
  return <Button
    testID={props.testID}
    accessibilityLabel={props.label}
    onPress={props.onPress}
    class="h-(--height-control) w-(--height-control) items-center justify-center rounded-(--radius-control) hover:bg-control-hover active:bg-control-pressed focus-visible:ring-2 ring-sidebar-ring"
  >
    <Text class="text-(length:--text-body) text-foreground">{props.children}</Text>
  </Button>;
}

function FontChoices(props: {
  fonts: readonly { id: string; label: string }[];
  selected: Signal<string>;
  testID: string;
  onSelect: (id: string) => void;
}) {
  return <View class="gap-0.5">
    <Each of={props.fonts}>{(font) =>
      <Button
        testID={`${props.testID}-${font().id}`}
        accessibilityLabel={`${font().label} font`}
        selected={props.selected() === font().id}
        onPress={() => props.onSelect(font().id)}
        class="h-(--height-tab) items-start justify-center rounded-(--radius-tab) px-2 hover:bg-tab-hover focus-visible:ring-2 ring-sidebar-ring"
        classList={{ "bg-tab-selected hover:bg-tab-selected": props.selected() === font().id }}
      >
        <Text style={{
          fontFamily: font().id,
          fontSize: token("text-body"),
          lineHeight: token("leading-body"),
          color: token(props.selected() === font().id ? "tab-selected-foreground" : "sidebar-foreground"),
        }}>{font().label}</Text>
      </Button>
    }</Each>
  </View>;
}
