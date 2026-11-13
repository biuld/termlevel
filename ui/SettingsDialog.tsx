import { keymap, Scroll, setColorScheme, signal, Text, token, View, type Signal } from "nsh";
import { Button } from "nsh/base";
import { Action, AppDialog } from "./controls";
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
  { label: "Find terminal", keys: "mod+k" },
  { label: "Toggle sidebar", keys: "mod+b" },
  { label: "Next terminal", keys: "ctrl+tab" },
  { label: "Split right", keys: "mod+d" },
  { label: "Split down", keys: "mod+shift+d" },
  { label: "Maximize / restore pane", keys: "mod+shift+enter" },
  { label: "Focus pane (use any arrow)", keys: "mod+alt+right" },
  { label: "Swap pane (use any arrow)", keys: "mod+alt+shift+right" },
] as const;

/** Two-column settings: a section list on the left, that section on the right. */
export function SettingsDialog(props: {
  open: boolean;
  dismiss: () => void;
  appearance: Signal<"system" | "light" | "dark">;
  fontSize: Signal<number>;
  version: string;
  preview: boolean;
}) {
  const page = signal<SettingsPage>("appearance");
  const lineHeight = () => props.fontSize() + token("line-gap-terminal");
  return <AppDialog testID="settings-dialog" open={props.open} label="Settings" title="Settings"
    frame="w-[560px]" dismiss={props.dismiss} closeLabel="Close settings">
    <View class="min-h-[280px] flex-row">
      <View class="w-[148px] gap-0.5 border-r border-border pr-2" role="tablist" accessibilityLabel="Settings sections">
        <Each of={pages}>{(item) =>
          <Button
            testID={`settings-${item().id}`}
            role="tab"
            selected={page() === item().id}
            accessibilityLabel={`${item().label} settings`}
            onPress={() => page.set(item().id)}
            class="h-(--height-tab) items-start justify-center rounded-(--radius-control) px-2 hover:bg-control-hover focus-visible:ring-2 ring-ring"
            classList={{ "bg-control-selected": page() === item().id }}
          >
            <Text class="font-medium" style={{
              fontSize: token("text-body"),
              lineHeight: token("leading-body"),
              color: token(page() === item().id ? "control-selected-foreground" : "foreground"),
            }}>{item().label}</Text>
          </Button>
        }</Each>
      </View>
      <Scroll class="max-h-[320px] min-w-0 flex-1 pl-4">
        <View class="gap-3">
          <Type role="title">{pages.find((item) => item.id === page())?.label}</Type>
          <If when={page() === "appearance"}>
            <View class="gap-2">
              <View class="flex-row gap-1 rounded-(--radius-tab) border border-border bg-background p-1">
                <Each of={["system", "light", "dark"] as const}>{(mode) =>
                  <Button
                    testID={`appearance-${mode()}`}
                    accessibilityLabel={`${mode()} appearance`}
                    selected={props.appearance() === mode()}
                    onPress={() => { props.appearance.set(mode()); setColorScheme(mode()); }}
                    class="h-(--height-control) flex-1 items-center justify-center rounded-(--radius-control) hover:bg-control-hover focus-visible:ring-2 ring-ring"
                    classList={{ "bg-control-selected": props.appearance() === mode() }}
                  >
                    <Text class="font-medium" style={{
                      fontSize: token("text-label"),
                      lineHeight: token("leading-label"),
                      color: token(props.appearance() === mode() ? "control-selected-foreground" : "muted-foreground"),
                    }}>{mode() === "system" ? "System" : mode() === "light" ? "Light" : "Dark"}</Text>
                  </Button>
                }</Each>
              </View>
              <Type role="caption">Applies immediately. Open terminals stay mounted.</Type>
            </View>
          </If>
          <If when={page() === "terminal"}>
            <View class="gap-3">
              <View class="flex-row items-center gap-3">
                <View class="min-w-0 flex-1 gap-0.5">
                  <Type role="body">Font size</Type>
                  <Type role="caption">Size is the terminal only. The family is shared with the toolbar path, which stays at body size.</Type>
                </View>
                <Action onPress={() => props.fontSize.update((n) => Math.max(10, n - 1))}>−</Action>
                <Type role="mono" class="w-12 text-center">{props.fontSize()} pt</Type>
                <Action onPress={() => props.fontSize.update((n) => Math.min(24, n + 1))}>+</Action>
              </View>
              <FontChoices fonts={monoFonts} selected={monoFont} testID="mono-font" onSelect={(id) => { monoFont.set(id); applyFonts(); }} />
              <View class="rounded-(--radius-control) bg-terminal-background px-3 py-2">
                <Text style={{ fontFamily: monoFont(), fontSize: props.fontSize(), lineHeight: lineHeight(), color: token("terminal-prompt") }}>~/Projects/termlevel</Text>
                <Text style={{ fontFamily: monoFont(), fontSize: props.fontSize(), lineHeight: lineHeight(), color: token("terminal-foreground") }}>echo typography</Text>
              </View>
            </View>
          </If>
          <If when={page() === "interface"}>
            <View class="gap-2">
              <Type role="caption">System is the platform font. The toolbar path uses the terminal family at body size, {token("text-body")} pt.</Type>
              <FontChoices fonts={interfaceFonts} selected={interfaceFont} testID="ui-font" onSelect={(id) => { interfaceFont.set(id); applyFonts(); }} />
              <View class="gap-1 rounded-(--radius-control) border border-border bg-background px-3 py-2">
                <Type role="title">Title {token("text-title")}</Type>
                <Type role="body">Body {token("text-body")} keeps names and settings readable.</Type>
                <Type role="body" family="mono" tone="muted-foreground">~/Projects/termlevel</Type>
                <Type role="label">Label {token("text-label")}</Type>
                <Type role="caption">Caption {token("text-caption")} for counts and hints.</Type>
              </View>
            </View>
          </If>
          <If when={page() === "shortcuts"}>
            <View class="gap-1.5">
              <Each of={shortcuts}>{(shortcut) => <View class="flex-row items-center justify-between gap-3">
                <Type role="body">{shortcut().label}</Type>
                <Type role="mono">{keymap.label(shortcut().keys)}</Type>
              </View>}</Each>
            </View>
          </If>
          <If when={page() === "about"}>
            <View class="gap-1">
              <Type role="body">Termlevel {props.version}</Type>
              <Type role="caption">{props.preview ? "Interactive UI preview. No shell is connected. Sessions and settings reset when the app restarts." : "Native terminal sessions powered by Alacritty. Closing a terminal ends its processes. Workspace layout and settings reset when the app restarts."}</Type>
            </View>
          </If>
        </View>
      </Scroll>
    </View>
  </AppDialog>;
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
        class="h-(--height-tab) items-start justify-center rounded-(--radius-control) px-2 hover:bg-control-hover focus-visible:ring-2 ring-ring"
        classList={{ "bg-control-selected": props.selected() === font().id }}
      >
        <Text style={{
          fontFamily: font().id,
          fontSize: token("text-body"),
          lineHeight: token("leading-body"),
          color: token(props.selected() === font().id ? "control-selected-foreground" : "foreground"),
        }}>{font().label}</Text>
      </Button>
    }</Each>
  </View>;
}
