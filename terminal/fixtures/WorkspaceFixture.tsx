import { mount } from "nsh";
import { WorkspaceApp } from "../../ui/WorkspaceApp";

// Exercise the production workspace with a deterministic real shell. User
// shell profiles/history are covered by separate native acceptance runs.
mount(() => <WorkspaceApp shell="/bin/sh" />);
