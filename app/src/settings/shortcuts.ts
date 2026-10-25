// Every keyboard shortcut, in one place: this list feeds the Shortcuts tab.
// Keep it in step with the menu in src-tauri/src/menu.rs and the README. The
// site and sheet tools' single keys come from the tools, so a key changes in one place.

import { HIGHLIGHT_KEY } from "../sheet/highlights/useHighlightTool";
import { CIRCLE_KEY, PAN_KEYS, POLYGON_KEY, RECTANGLE_KEY } from "../site/SitePanel";
import { SECTION_KEY } from "../sheet/section/useSectionTool";
import { VIEW_POINT_KEY } from "../sheet/viewpoint/useViewPointTool";

export const SHORTCUTS: { title: string; items: { label: string; keys: string[] }[] }[] = [
  {
    title: "Everywhere",
    items: [
      { label: "New project", keys: ["mod+n"] },
      { label: "All projects", keys: ["mod+shift+h"] },
      { label: "Settings", keys: ["mod+,"] },
      { label: "Keyboard shortcuts", keys: ["mod+/"] },
    ],
  },
  {
    title: "Projects",
    items: [
      { label: "Open the selected project", keys: ["enter", "mod+o"] },
      { label: "Move to Recently Deleted", keys: ["delete"] },
      { label: "Select the next or previous", keys: ["←", "→"] },
    ],
  },
  {
    title: "Site",
    items: [
      { label: "Circle", keys: [CIRCLE_KEY] },
      { label: "Rectangle", keys: [RECTANGLE_KEY] },
      { label: "Polygon", keys: [POLYGON_KEY] },
      { label: "Move the map", keys: PAN_KEYS },
      { label: "Finish the polygon", keys: ["enter"] },
      { label: "Remove the last corner", keys: ["delete"] },
      { label: "Delete the selected shape", keys: ["delete"] },
      { label: "Stop drawing or deselect", keys: ["escape"] },
      { label: "Undo", keys: ["mod+z"] },
      { label: "Redo", keys: ["mod+shift+z"] },
      { label: "Zoom in or out", keys: ["mod+=", "mod+-"] },
      { label: "Fit the site", keys: ["mod+0"] },
    ],
  },
  {
    title: "Sheet",
    items: [
      { label: "Previous or next diagram", keys: ["↑", "↓"] },
      { label: "Previous or next variant, like PM10 or PM2.5", keys: ["←", "→"] },
      { label: "Show or hide the diagrams", keys: ["d"] },
      { label: "Previous or next palette", keys: ["[", "]"] },
      { label: "Show or hide the inspector", keys: ["i"] },
      { label: "Add a text box", keys: ["t"] },
      { label: "Edit the selected text", keys: ["enter"] },
      { label: "Nudge the selected text, by 10 mm with Shift", keys: ["←", "→"] },
      { label: "Delete the selected text box", keys: ["delete"] },
      { label: "Next or previous sheet tab", keys: ["ctrl+tab", "ctrl+shift+tab"] },
      { label: "Next or previous sheet tab, as in spreadsheets", keys: ["ctrl+pagedown", "ctrl+pageup"] },
      { label: "Next or previous sheet tab on a Mac", keys: ["mod+alt+→", "mod+alt+←"] },
      { label: "Rename the focused sheet tab", keys: ["enter"] },
      { label: "Move the focused sheet tab", keys: ["alt+←", "alt+→"] },
      { label: "Edit the selected pin's label", keys: ["enter"] },
      { label: "Nudge the selected pin, by 10 mm with Shift", keys: ["←", "→"] },
      { label: "Make the selected pin larger or smaller", keys: ["plus", "-"] },
      { label: "Delete the selected pin", keys: ["delete"] },
      { label: "Draw a highlight, or stop drawing", keys: [HIGHLIGHT_KEY] },
      { label: "Finish a polygon highlight", keys: ["enter"] },
      { label: "Remove the polygon's last corner", keys: ["delete"] },
      { label: "Edit the selected highlight's label", keys: ["enter"] },
      { label: "Nudge the selected highlight, by 10 mm with Shift", keys: ["←", "→"] },
      { label: "Grow or shrink the selected highlight", keys: ["plus", "-"] },
      { label: "Delete the selected highlight", keys: ["delete"] },
      { label: "Draw a section line, or stop drawing", keys: [SECTION_KEY] },
      { label: "Delete the selected section line", keys: ["delete"] },
      { label: "Place the View diagram's viewpoint", keys: [VIEW_POINT_KEY] },
      { label: "Stop drawing or deselect", keys: ["escape"] },
      { label: "Undo", keys: ["mod+z"] },
      { label: "Redo", keys: ["mod+shift+z"] },
      { label: "Export", keys: ["mod+e"] },
      { label: "Zoom in or out", keys: ["mod+=", "mod+-"] },
      { label: "Fit the sheet", keys: ["mod+0"] },
    ],
  },
  {
    title: "Report",
    items: [
      { label: "Add a text block to the page", keys: ["t"] },
      { label: "Finish editing a text block, a new line with Shift", keys: ["enter"] },
      { label: "Nudge the selected block, by 10 mm with Shift", keys: ["←", "→"] },
      { label: "Duplicate the selected block", keys: ["mod+d"] },
      { label: "Bring forward or send backward", keys: ["mod+]", "mod+["] },
      { label: "Delete the selected block", keys: ["delete"] },
      { label: "Deselect", keys: ["escape"] },
      { label: "Undo", keys: ["mod+z"] },
      { label: "Redo", keys: ["mod+shift+z"] },
      { label: "Export", keys: ["mod+e"] },
      { label: "Zoom in or out", keys: ["mod+=", "mod+-"] },
      { label: "Fit the pages", keys: ["mod+0"] },
    ],
  },
];
