// Every keyboard shortcut, in one place: this list feeds the Shortcuts tab.
// Keep it in step with the menu in src-tauri/src/menu.rs and the README.

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
      { label: "Circle", keys: ["c"] },
      { label: "Rectangle", keys: ["r"] },
      { label: "Polygon", keys: ["p"] },
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
      { label: "Diagram 1 to 9", keys: ["1", "9"] },
      { label: "Previous or next palette", keys: ["[", "]"] },
      { label: "Show or hide the inspector", keys: ["i"] },
      { label: "Add a text box", keys: ["t"] },
      { label: "Edit the selected text", keys: ["enter"] },
      { label: "Nudge the selected text, by 10 mm with Shift", keys: ["←", "→"] },
      { label: "Delete the selected text box", keys: ["delete"] },
      { label: "Undo", keys: ["mod+z"] },
      { label: "Redo", keys: ["mod+shift+z"] },
      { label: "Export", keys: ["mod+e"] },
      { label: "Zoom in or out", keys: ["mod+=", "mod+-"] },
      { label: "Fit the sheet", keys: ["mod+0"] },
    ],
  },
];
