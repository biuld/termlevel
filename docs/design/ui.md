# UI and Theme

Termlevel uses a unified window bar with independent panes. Workspace ownership is described in the [workspace architecture](../architecture/workspace.md), the terminal platform boundary in the [terminal architecture](../architecture/terminal.md), terminal rendering in [terminal.md](terminal.md), multi-pane interaction in [tiling.md](tiling.md), status in [osc-status.md](osc-status.md), notification history in the [notifications architecture](../architecture/notifications.md), and validation commands in [development.md](../development.md).

## Design direction

The window base plate carries one top window bar. The sidebar and terminal sit below it as rounded, docked panels; the floating feel comes from whitespace and surface lightness differences.

- Sidebar: default 208 pt, adjustable 176–288 pt, collapsed 44 pt.
- One unified 40 pt window bar. System window buttons and the sidebar toggle are fixed on the left; the app name is centered. Expanding, collapsing or hovering the sidebar never moves window controls. Blank areas of the bar drag the window.
- Pane tops meet the window bar directly with no extra spacing. Content left/right/bottom margins and pane gaps are 8 pt. Sidebar and terminal align top and bottom with a 12 pt radius. The sidebar top is a 36 pt Sessions heading; collapsed it shows a terminal icon. The terminal toolbar is also 36 pt. The gap between columns carries the resize hit area and follows sidebar width during a drag.
- Pinned and normal tabs are both 28 pt single-line rows showing only icon and name. Pinned cards, directory subtitles and large icon pedestals are removed. OSC 7501 program status replaces the default terminal icon; OSC 9;4 progress is a 2 pt line at the bottom of the tab; the title still comes from OSC 0 / OSC 2. Status and real-progress UI/UX are in [osc-status.md](osc-status.md): blocked / error never synthesize a full bar, viewing only acknowledges read state and does not clear blocked, and tooltip, accessibility and the line use one progress summary.

## Tabs and groups

Groups use a solid color marker, group name and thin guide line. The marker has a 3 pt rounded corner at bottom-left and is semicircular elsewhere; it contains no folder glyph. Expanded markers are 14 pt. Titles are 12 pt medium in neutral body color, single-line with ellipsis, and the row is 28 pt high. Expanded groups tile directly on the sidebar without a group card background or outer radius; hovering a group header reveals only its action buttons rather than tinting the whole row. A selected terminal keeps one neutral selection layer. Solid `group-*` colors are used for group markers and for the terminal icon of a selected tab inside a group; the group name stays neutral. Members keep a low-contrast `group-*-line` on their left, with `gap-group-guide` (6 pt) between the guide's inner edge and the tab's left edge; no colored vertical bar is layered inside the tab. The count moves into the group header tooltip, and the more button only shows on hover, keyboard focus or while editing.

Vertical tab dragging reorders tabs and moves them into another group. Dropping on a tab in an existing group joins that group; dropping on an ungrouped tab forms a new group with it, taking the path and group name from the target tab without opening the group editor. A highlighted group header means the drop joins that group, a highlighted tab means grouping with that tab, and a line between tabs shows the insertion point; releasing outside the sidebar or sliding horizontally cancels. A collapsed group header is also a drop target and expands on drop. Dragging never changes session ID. A pinned tab floats at the top of its group or Ungrouped and reorders there, and cross-group moves retain pinned state.

When the sidebar is collapsed, group backgrounds are not painted. The group marker stays the expanded 14 pt marker, centered in the row, and a low-contrast 1 pt color line on the left threads the whole group. A collapsed group keeps its currently selected tab; other tabs appear through a circular `+N` count entry, and clicking the count expands the group. When switching to a tab outside the group, the count includes every hidden tab.

Hovering over the collapsed sidebar expands it as an overlay to the saved sidebar width, covering the terminal instead of squeezing content; it retracts on exit. The sidebar stays expanded while the group editor overlay is open, and manual expansion still uses the fixed split layout. The collapsed state, including the hover overlay, disables divider dragging, uses the normal cursor and shows no drag hover highlight; only manual fixed expansion allows width adjustment.

Search terminals, new group and settings live in the window title bar. The sidebar bottom shows no session count and draws no horizontal separator. New non-split terminals use ⌘T or the sidebar list entry; an empty workspace still shows a new button. Terminals created from the sidebar and shortcuts stay outside the current group; only group-internal creation and new-group creation place a terminal inside a group.

## Terminal pane and toolbar

A terminal pane contains a session toolbar and body with the same background, clipped to the pane's rounded corners. Body padding is 16 pt left, right and bottom, with no top padding; scrolling content meets the toolbar's lower edge directly, and the scroll grid and native Input share one content coordinate origin. There is no persistent status bar; only process exit or error shows status.

The toolbar shows the terminal icon and current directory on the left, switching to the session name in narrow windows. On the right it offers content search, split and more actions. Search opens the current pane's find bar and is the same entry as ⌘F and the menu's Find in terminal. Toolbar content aligns with the body's left padding. Window dragging belongs to the unified top bar. Preview identity remains explicit in the terminal welcome message and the settings note.

The UI defaults to the platform font `system-ui`. `SF Pro Text` is not an installed family and must not be used as `font-sans`. Settings choose an installed UI font and the monospace font shared by terminal and toolbar paths. The toolbar path uses body size with the terminal monospace font and has no separate size; shortcuts use the chosen monospace font at caption size. Type roles follow the table below. Icon buttons are 26 pt, action buttons 28 pt and inputs 30 pt.

## Dialogs and settings

Centered dialogs use the NSH 0.6.0 `Dialog`. The runtime only supplies the layer and close behavior; the application paints the appearance: the base is the window `chrome`, with the same radius as a pane, a `sidebar-border` ring, a light shadow and a uniform `bg-scrim` beneath. There is no glass and no white popover card. A short title sits on the left of the 36 pt title bar with the close button on the right. Confirm-delete puts the question in the content area and uses `alertdialog`. The primary action uses the selected-tab color, cancel is text without a background, and delete keeps the destructive color. Inputs are inset fields in the sidebar color. Escape and clicking the scrim close. The group editor remains an `Overlay` anchored to the group header, not this dialog.

Settings reuse the window's two surface layers. Sections sit directly on the base plate on the left, with selection styled like a sidebar tab. The right side is one `sidebar` rounded panel showing only the current section. Sections are Appearance, Terminal, Shortcuts and About. Appearance collects the color theme, working status indicator and UI font; the color theme offers System / Light / Dark, defaults to following the system, and switching appearance preserves terminal instances. Terminal selects the monospace font at size 10–24 pt, defaulting to the bundled JetBrains Mono → Sarasa Term SC → system monospace chain at 13 pt, with line height equal to size plus `line-gap-terminal`. Scroll history defaults to 10,000 lines and adjusts from 1,000 to 100,000 in 1,000-line steps, applying immediately to open terminals; reducing it drops the oldest lines. Toolbar paths use the same monospace font at body size. UI font choices are System, Helvetica Neue, Avenir Next, Gill Sans, Futura and Georgia; type sizes still come from the theme. The Shortcuts page lists the bindings actually in effect. Workspace structure and settings persist through NSH Storage (`~/Library/Application Support/app.termlevel.desktop/storage.json`); appearance, fonts, font size, scroll history, sidebar width and tab groups restore after restart, while clipboard read permission still resets per the security design.

## Type roles

Interface text uses the roles in `ui/shared/type.tsx`; components do not define their own sizes. Sizes and line heights live only in `theme.json`. Families are selected in settings; the default UI family is `system-ui`, and the default monospace family is the bundled JetBrains Mono → Sarasa Term SC → system monospace chain.

| Role | Size | Line height | Weight | Use |
|---|---|---|---|---|
| `heading` | 18 | 22 | medium | empty states |
| `title` | 15 | 20 | semibold | dialog titles |
| `body` | 12 | 16 | regular | tab names, settings rows, buttons, group names (medium) |
| `label` | 11 | 15 | medium | settings sections, segmented controls |
| `caption` | 10 | 13 | regular | descriptions, counts |
| `mono` | 10 | 13 | chosen monospace | shortcuts and size readouts in settings |
| Terminal | 13 | size + 8 | chosen monospace | default terminal size and font, adjusted in settings |

No text is smaller than 10 pt. The former `text-micro`, which matched caption, is removed.

## Fleet Dark / Light palette

The palette follows the Fleet Islands UI Dark / Light direction. Values are mapped from [the Fleetty light and dark theme sources](https://github.com/erenozen/Fleetty/tree/d206733e56c1b16d1fa5f09e322ae8ec8d13a25e/themes) (MIT). This is a public port whose repository describes the source as a theme exported from Fleet; it is not treated as an official complete theme distribution. [JetBrains' Islands UI notes](https://blog.jetbrains.com/fleet/2024/12/introducing-fleets-new-islands-ui/) provide the design background.

| Use | Dark | Light |
|---|---|---|
| Window base plate | `#090909` | `#eeeff0` |
| Sidebar / terminal pane | `#18191b` | `#ffffff` |
| Primary text / terminal default text | `#e0e1e4` | `#090909` |
| Secondary text | `#909192` | `#747474` |
| Popover | `#252629` | `#ffffff` |
| Primary action | `#0870e4` | `#0870e4` |
| Focus | `#2a7deb` | `#2a7deb` |
| Destructive action | `#ec5d6f` | `#c72c49` |

Selection, hover, borders and secondary text use the corresponding semantic colors from the source. The source's transparent overlays are pre-composited on the pane background, and the app still uses opaque surfaces. The nine group markers use the same blue, purple, cyan, orange, yellow, pink, green, gray and red set; the collapsed state hides the count entry background and guide line composited from them. Preview hints use Fleet's cyan keyword and blue link colors.

The mapping covers the workspace, controls, groups and terminal default foreground, background and cursor. The ANSI 16 colors remain the NSH fixed palette; 256-color and truecolor keep program semantics. Editor syntax highlighting is not used as a rule for recoloring terminal output.

## Surfaces and elevation

Custom application UI uses solid surfaces throughout. It does not use Liquid Glass, background blur or platform translucent materials.

- The window base plate uses `chrome`; it is darker than panes in dark mode and light gray in light mode. Fixed-expanded, collapsed and hover-expanded sidebars all use opaque `sidebar`, close to the terminal surface. Hover expansion does not change the base color; a thin border and light shadow express the overlay. Groups tile on the sidebar, and color is limited to group markers, member guide lines and the active terminal icon; no group panel background is layered underneath.
- Fixed panels are separated by 8 pt of whitespace and cast no shadow. The resize indicator line appears only while hovering or dragging in the fixed-expanded state; collapsed and temporary hover expansion disable resizing.
- The session toolbar uses the same background as the terminal. The directory and buttons sit directly on the toolbar with no capsule base; buttons keep hover, pressed and keyboard-focus feedback.
- The group editor overlay and tooltips use opaque `popover` with a `border` thin border, `radius-overlay` corners and a light shadow.
- The selected tab keeps a solid capsule, and the terminal keeps `terminal-background`. The uniform scrim under a dialog expresses modality only and does not blur the background.

## Notification content

Notification history uses single-column, content-driven rows; each notification is not nested in another floating card. A 32 pt tinted base on the left carries an 18 pt semantic icon: shield exclamation for waiting for approval, question mark for a question, lock for sign-in, circled x for failure, and information for ordinary messages. Colors use notification-info / notification-warning / destructive and their muted backgrounds; shape and category text together distinguish the type. When the program provides no image attachment, no brand icon or task screenshot is fabricated.

Notification main text and context both use the UI body role (12 / 16). Main text stands out through semibold weight, color and spacing; it is not enlarged to the dialog title role. The event title or task title remains as regular-weight context. Program notifications prefer the structured message, then the task title; ordinary notifications prefer the body and fall back to the title. Source, program name, category and HH:MM time use 10 pt secondary text, and a repeat count uses a small capsule. Actions to return to the source, expand / collapse and remove sit on their own at the bottom rather than competing with the main text on the same line.

Long content defaults to a bounded preview: 100 Unicode scalars for the main text and 160 for the context, each keeping the first three explicit lines. Show more appears only when content was omitted; expansion uses the original full historical text, and a long main text switches to regular weight for continuous reading. Show less collapses in place; expanding or collapsing does not change size or line height. Program messages are not interpreted as approval / sign-in buttons or rich text. When the source session closes, content is retained and the jump action is disabled.

The panel is 420 pt wide, with a maximum width of window width minus 16 pt. History height is the minimum of the measured total height of the first three items, 384 pt and the available window height. Separators, the scrollbar and a more-content hint remain. Long-content expansion still obeys the history height cap. The empty state uses a centered bell base, a short title and a description.

## Group editor

The group editor follows Firefox's group-edit overlay. Right-clicking a group header or clicking the more button opens `GroupEditor` next to the group header; there is no native submenu for color selection and no separate rename dialog.

- The top row has nine color dots: blue, purple, cyan, orange, yellow, pink, green, gray and red. The current color has a selection ring, and colors still come from the theme.
- Each group records one path. The group name defaults to the last path segment and can be renamed in the editor, which also shows the full path. An unnamed group shows `Untitled Group`.
- Creating a group first opens path selection, listing directories of existing terminals and allowing the system folder panel to choose another. The group is created only after a path is chosen, and a terminal opens at that path; cancelling or closing the picker creates nothing. The group editor does not open automatically. A random color is chosen from the nine group colors that are not yet in use; once all nine are used, a color is chosen randomly from the full set. New terminals created inside the group start from the group path. Sidebar and shortcut terminals remain outside the group and start from the current terminal's path.
- Dropping onto an ungrouped tab to form a group takes the path and name from that target tab, again with a random color.
- The editor provides new terminal in group, ungroup and delete group. Ungrouping keeps the terminals; deleting a group requires confirmation and closes the group's terminals.
- Enter or Escape closes the editor, and clicking outside also closes it. Selecting a color keeps input focus; closing does not roll back an already changed name or color.
- The overlay is anchored to the whole group header and stays positioned as the header scrolls, opens and collapses. The NSH 0.6.0 `Overlay` handles screen-edge constraints; width is 280 pt with compact density.
- Browser links, AI recommendations, save group and multi-window operations are not added as placeholder entries in the current prototype.

## Theme configuration and usage

`nsh.json` points `theme` at `theme.json` and sets `appearance` to `system`.

`theme.json` is the single entry point for application design values:

| Category | Tokens |
|---|---|
| Base | `background`, `foreground`, `primary`, `border`, `muted-foreground` |
| Window and navigation | `chrome`, `sidebar-*`, `height-titlebar`, `width-sidebar`, `gap-pane` |
| Tabs and controls | `tab-selected-*`, `tab-hover`, `control-hover`, `control-pressed`, `control-selected-*` |
| Groups | `group-purple` and the others, plus their `-muted`, `-line` and `gap-group-guide` |
| Terminal | `terminal-background`, `terminal-foreground`, `terminal-cursor`, `font-mono` |
| Fonts and density | `font-sans`, `text-*`, `leading-*`, `height-*`, `padding-terminal` |
| Radius | `radius-control`, `radius-tab`, `radius-panel`, `radius-dialog`, `radius-overlay` |

`cssVars.theme` holds shared fonts and sizes, while `cssVars.light` and `dark` hold complete palettes. Custom token names must match in both appearances.

TSX static styles use semantic classes:

```tsx
<View class="bg-sidebar rounded-(--radius-tab)">
  <Text class="text-sidebar-foreground text-(length:--text-body)">Terminal</Text>
</View>
```

Canvas and dynamic styles read `token()` rather than copying literal color values. Group data stores stable identifiers such as `purple` / `blue`, and `ui/shared/theme.ts` maps them to theme tokens. Existing groups update when the theme changes.

Every centered dialog sets `m-auto` in the `Dialog` `class`. That property applies to the dialog body; NSH does not center fixed-width dialogs automatically.

The theme does not own session or workspace state. Switching appearance goes through NSH `setColorScheme()` and does not unmount or recreate terminals. The UI theme controls the terminal's default text, background, cursor and font, while ANSI colors chosen by programs keep program semantics. Preview branding and prompts derive ANSI colors from the theme; truecolor characters already written are not rewritten when the theme changes.

After changing the theme, run `./scripts/nsh build` to generate token type declarations, then run `./scripts/nsh check`. Do not edit `.nsh/types` by hand. Native platform visuals, input methods and selection still require separate acceptance.
