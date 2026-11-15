import { setColorScheme, signal, View } from "nsh";
import { Runtime } from "termlevel/native";
import { WorkspaceApp } from "./WorkspaceApp";
import { applyFonts, interfaceFont, interfaceFonts, monoFont, monoFonts } from "../shared/fonts";
import { loadWorkspaceSnapshot } from "../workspace/persistence";
import type { WorkspaceSnapshot } from "../workspace/workspace-snapshot";

/**
 * Loads the saved workspace before mounting the app, so restored fonts and
 * appearance apply on the first frame. A first launch has no document: the
 * workspace starts empty instead of opening a shell by itself.
 */
export function WorkspaceRoot() {
  // Preview stays synchronous and deterministic: it never reads real storage.
  if (Runtime.previewMode()) return <WorkspaceApp />;

  const loaded = signal<{ snapshot: WorkspaceSnapshot | null } | null>(null);
  void loadWorkspaceSnapshot().then((snapshot) => {
    if (loaded()) return;
    if (snapshot) {
      if (snapshot.interfaceFont && interfaceFonts.some((font) => font.id === snapshot.interfaceFont)) {
        interfaceFont.set(snapshot.interfaceFont);
      }
      if (snapshot.monoFont && monoFonts.some((font) => font.id === snapshot.monoFont)) {
        monoFont.set(snapshot.monoFont);
      }
      setColorScheme(snapshot.appearance);
      applyFonts();
    }
    loaded.set({ snapshot });
  });

  return <View class="flex-1 bg-chrome">
    <If when={loaded()}>{(value) => <WorkspaceApp snapshot={value().snapshot} persist />}</If>
  </View>;
}
