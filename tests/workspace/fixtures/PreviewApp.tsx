import { mount } from "nsh";
import { WorkspaceApp } from "../../../ui/app/WorkspaceApp";
import { previewGit } from "./preview/git";
import { PreviewTerminal } from "./preview/Terminal";

mount(() => <WorkspaceApp git={previewGit} previewSurface={PreviewTerminal} />);
