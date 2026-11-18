import { colorScheme, Image } from "nsh";
import codex from "../../assets/agents/codex.svg";
import claude from "../../assets/agents/claude-code.svg";
import grok from "../../assets/agents/grok.svg";
import pi from "../../assets/agents/pi.svg";
import opencodeDark from "../../assets/agents/opencode-dark.svg";
import opencodeLight from "../../assets/agents/opencode-light.svg";

const agents = {
  codex: { name: "Codex", image: codex },
  "claude-code": { name: "Claude Code", image: claude },
  grok: { name: "Grok", image: grok },
  "grok-build": { name: "Grok", image: grok },
  pi: { name: "Pi", image: pi },
  opencode: { name: "OpenCode", image: opencodeDark },
} as const;

export function agentForApp(app?: string | null) {
  return app && Object.prototype.hasOwnProperty.call(agents, app)
    ? agents[app as keyof typeof agents] : null;
}

export function AgentIcon(props: { app: string }) {
  const agent = () => agentForApp(props.app);
  const source = () => props.app === "opencode" && colorScheme() === "light" ? opencodeLight : agent()?.image;
  return <Image src={source()} style={{ width: 15, height: 15, objectFit: "contain", pointerEvents: "none" }} />;
}
