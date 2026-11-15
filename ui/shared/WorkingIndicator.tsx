import { createMotion, effect, reduceMotion, token, View, type Motion } from "nsh";

/** The working-state motion stored with the workspace. */
export type WorkingIndicatorKind = "bars" | "vinyl" | "wave" | "beat";

const frameRate = 30;

/**
 * The status icon shown while a program reports working. A choice from
 * Settings picks the motion; Reduce Motion shows the same shape at rest.
 */
export function WorkingIndicator(props: { kind: WorkingIndicatorKind; size?: number; color?: string; active?: boolean }) {
  const size = props.size ?? 13;
  return <View style={{ width: size, height: size, alignItems: "center", justifyContent: "center", flexShrink: 0 }}>
    <If when={reduceMotion()}>
      <Static kind={props.kind} size={size} color={props.color ?? token("primary")} />
    </If>
    <If when={!reduceMotion()}>
      <If when={props.kind === "bars"}><Bars size={size} color={props.color ?? token("primary")} active={props.active ?? true} /></If>
      <If when={props.kind === "vinyl"}><Vinyl size={size} color={props.color ?? token("primary")} active={props.active ?? true} /></If>
      <If when={props.kind === "wave"}><Wave size={size} color={props.color ?? token("primary")} active={props.active ?? true} /></If>
      <If when={props.kind === "beat"}><Beat size={size} color={props.color ?? token("primary")} active={props.active ?? true} /></If>
    </If>
  </View>;
}

function Bars(props: { size: number; color: string; active: boolean }) {
  const unit = props.size / 13;
  const motions = [createMotion(), createMotion(), createMotion()];
  const specs = [
    { from: 0.3, to: 1, duration: 640 },
    { from: 0.55, to: 0.95, duration: 480 },
    { from: 0.2, to: 0.8, duration: 580 },
  ];
  effect(() => {
    if (!props.active) return;
    motions.forEach((motion, index) => {
      const spec = specs[index];
      void motion.play({
        keyframes: { scaleY: [spec.from, spec.to] },
        duration: spec.duration,
        direction: "alternate",
        iterations: "infinite",
        easing: "easeInOut",
        frameRate,
      });
    });
    return () => motions.forEach((motion) => motion.stop());
  });
  return <View style={{
    width: props.size, height: props.size, flexDirection: "row",
    alignItems: "flex-end", justifyContent: "center", gap: 1.6 * unit,
  }}>
    <View style={bar(motions[0], unit, props.color)} />
    <View style={bar(motions[1], unit, props.color)} />
    <View style={bar(motions[2], unit, props.color)} />
  </View>;
}

function bar(motion: Motion, unit: number, color: string) {
  return {
    width: 2 * unit,
    height: 10 * unit,
    borderRadius: 1 * unit,
    backgroundColor: color,
    transformOrigin: "50% 100%",
    ...motion.style,
  };
}

function Vinyl(props: { size: number; color: string; active: boolean }) {
  const unit = props.size / 13;
  const motion = createMotion();
  effect(() => {
    if (!props.active) return;
    void motion.play({
      keyframes: { rotate: [0, 360] },
      duration: 2200,
      iterations: "infinite",
      easing: "linear",
      frameRate,
    });
    return () => motion.stop();
  });
  return <View style={{ width: props.size, height: props.size, alignItems: "center", justifyContent: "center" }}>
    <View style={{
      width: 10 * unit,
      height: 10 * unit,
      borderRadius: 5 * unit,
      borderWidth: 1 * unit,
      borderColor: props.color,
      alignItems: "center",
      justifyContent: "center",
      ...motion.style,
    }}>
      <View style={{ width: 2.6 * unit, height: 2.6 * unit, borderRadius: 1.3 * unit, backgroundColor: props.color }} />
      <View style={{
        position: "absolute",
        top: 0,
        bottom: "50%",
        left: "50%",
        marginLeft: -0.5 * unit,
        width: 1 * unit,
        borderRadius: 0.5 * unit,
        backgroundColor: props.color,
      }} />
    </View>
  </View>;
}

function Wave(props: { size: number; color: string; active: boolean }) {
  const unit = props.size / 13;
  const amplitude = 0.9 * unit;
  const motions = [createMotion(), createMotion(), createMotion()];
  effect(() => {
    if (!props.active) return;
    motions.forEach((motion, index) => {
      void motion.play({
        keyframes: { translateY: [amplitude, -amplitude] },
        duration: 900,
        delay: index * 150,
        direction: "alternate",
        iterations: "infinite",
        easing: "easeInOut",
        frameRate,
      });
    });
    return () => motions.forEach((motion) => motion.stop());
  });
  return <View style={{
    width: props.size, height: props.size, flexDirection: "row",
    alignItems: "center", justifyContent: "center", gap: 2.4 * unit,
  }}>
    <View style={dot(motions[0], unit, props.color)} />
    <View style={dot(motions[1], unit, props.color)} />
    <View style={dot(motions[2], unit, props.color)} />
  </View>;
}

function dot(motion: Motion, unit: number, color: string) {
  return {
    width: 2.6 * unit,
    height: 2.6 * unit,
    borderRadius: 1.3 * unit,
    backgroundColor: color,
    ...motion.style,
  };
}

function Beat(props: { size: number; color: string; active: boolean }) {
  const unit = props.size / 13;
  const motion = createMotion();
  effect(() => {
    if (!props.active) return;
    void motion.play({
      keyframes: [
        { offset: 0, scale: 0.45, opacity: 0.85 },
        { offset: 0.7, scale: 0.85, opacity: 0.15 },
        { offset: 1, scale: 1, opacity: 0 },
      ],
      duration: 1150,
      iterations: "infinite",
      easing: "easeOut",
      frameRate,
    });
    return () => motion.stop();
  });
  return <View style={{
    width: props.size, height: props.size, alignItems: "center", justifyContent: "center",
  }}>
    <View style={{
      position: "absolute",
      width: 9 * unit,
      height: 9 * unit,
      borderRadius: 4.5 * unit,
      borderWidth: 1 * unit,
      borderColor: props.color,
      ...motion.style,
    }} />
    <View style={{
      width: 4 * unit,
      height: 4 * unit,
      borderRadius: 2 * unit,
      backgroundColor: props.color,
    }} />
  </View>;
}

/** The same shapes with the motions parked: a still, recognizable working glyph. */
function Static(props: { kind: WorkingIndicatorKind; size: number; color: string }) {
  const unit = props.size / 13;
  return <View style={{
    width: props.size, height: props.size, flexDirection: "row",
    alignItems: "center", justifyContent: "center",
  }}>
    <If when={props.kind === "bars"}>
      <View style={{ flexDirection: "row", alignItems: "flex-end", gap: 1.6 * unit, height: 10 * unit }}>
        <View style={{ width: 2 * unit, height: 4.5 * unit, borderRadius: unit, backgroundColor: props.color }} />
        <View style={{ width: 2 * unit, height: 10 * unit, borderRadius: unit, backgroundColor: props.color }} />
        <View style={{ width: 2 * unit, height: 7 * unit, borderRadius: unit, backgroundColor: props.color }} />
      </View>
    </If>
    <If when={props.kind === "vinyl"}>
      <View style={{
        width: 10 * unit, height: 10 * unit, borderRadius: 5 * unit,
        borderWidth: 1 * unit, borderColor: props.color,
        alignItems: "center", justifyContent: "center",
      }}>
        <View style={{ width: 2.6 * unit, height: 2.6 * unit, borderRadius: 1.3 * unit, backgroundColor: props.color }} />
        <View style={{
          position: "absolute", top: 0, bottom: "50%", left: "50%",
          marginLeft: -0.5 * unit, width: 1 * unit, borderRadius: 0.5 * unit,
          backgroundColor: props.color,
        }} />
      </View>
    </If>
    <If when={props.kind === "wave"}>
      <View style={{ flexDirection: "row", alignItems: "center", gap: 2.4 * unit }}>
        <View style={{ width: 2.6 * unit, height: 2.6 * unit, borderRadius: 1.3 * unit, backgroundColor: props.color, translate: [0, 0.9 * unit] }} />
        <View style={{ width: 2.6 * unit, height: 2.6 * unit, borderRadius: 1.3 * unit, backgroundColor: props.color }} />
        <View style={{ width: 2.6 * unit, height: 2.6 * unit, borderRadius: 1.3 * unit, backgroundColor: props.color, translate: [0, -0.9 * unit] }} />
      </View>
    </If>
    <If when={props.kind === "beat"}>
      <View style={{ width: props.size, height: props.size, alignItems: "center", justifyContent: "center" }}>
        <View style={{
          position: "absolute",
          width: 9 * unit, height: 9 * unit, borderRadius: 4.5 * unit,
          borderWidth: 1 * unit, borderColor: props.color, opacity: 0.55,
        }} />
        <View style={{ width: 4 * unit, height: 4 * unit, borderRadius: 2 * unit, backgroundColor: props.color }} />
      </View>
    </If>
  </View>;
}
