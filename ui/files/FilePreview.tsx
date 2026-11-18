import { effect, Image, Scroll, signal, Video, View } from "nsh";
import { TextView } from "nsh/base";
import { Action } from "../shared/controls";
import { Type } from "../shared/type";

export type PreviewKind = "image" | "video" | "audio" | "markdown" | "html" | "text";

export function previewKind(path: string): PreviewKind {
  const name = path.toLowerCase();
  const extension = name.slice(name.lastIndexOf(".") + 1);
  if (["png", "jpg", "jpeg", "gif", "webp", "avif", "heic", "heif", "bmp", "tif", "tiff", "ico", "svg"].includes(extension)) return "image";
  if (["mp4", "m4v", "mov", "webm", "mkv", "avi"].includes(extension)) return "video";
  if (["mp3", "m4a", "aac", "wav", "ogg", "oga", "opus", "flac", "aif", "aiff", "caf"].includes(extension)) return "audio";
  if (["md", "markdown", "mdown", "mkd", "mkdn"].includes(extension)) return "markdown";
  if (["html", "htm"].includes(extension)) return "html";
  return "text";
}

export function FilePreview(props: {
  kind: PreviewKind;
  url: string;
  text: string;
  openWith: () => void;
}) {
  const error = signal("");
  const loaded = signal(false);
  effect(() => { props.url; props.kind; error.set(""); loaded.set(false); });
  return <View testID="file-preview" class="min-h-0 flex-1">
    <If when={!!error()}>
      <View class="flex-1 items-center justify-center gap-2 px-6">
        <Type role="body" tone="muted-foreground">This file could not be previewed.</Type>
        <Action onPress={props.openWith}>Open with default app</Action>
      </View>
    </If>
    <If when={!error() && props.kind === "image"}>
      <Image testID="file-image-preview" src={props.url} class="h-full w-full"
        accessibilityLabel={loaded() ? "Image preview loaded" : "Loading image preview"}
        style={{ objectFit: "contain" }} onLoad={() => loaded.set(true)} onError={() => error.set("image")} />
    </If>
    <If when={!error() && (props.kind === "video" || props.kind === "audio")}>
      <View class="min-h-0 flex-1 items-center justify-center">
        <Video testID="file-media-preview" src={props.url} paused={true} controls={true}
          accessibilityLabel={loaded() ? "Media preview loaded" : "Loading media preview"}
          class="w-full" style={{ height: props.kind === "audio" ? 120 : "100%" }}
          onLoad={() => loaded.set(true)} onError={() => error.set("media")} />
      </View>
    </If>
    <If when={!error() && (props.kind === "markdown" || props.kind === "html")}>
      <Scroll class="min-h-0 flex-1" scrollbar={true}>
        <TextView testID="file-text-preview" class="p-4" selectable={true}
          markdown={props.kind === "markdown" ? props.text : undefined}
          html={props.kind === "html" ? props.text : undefined} />
      </Scroll>
    </If>
  </View>;
}
