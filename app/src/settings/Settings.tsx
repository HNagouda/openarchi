// Settings: appearance and the library folder, how to connect an assistant,
// and every keyboard shortcut in one list.

import { useEffect, useState } from "react";
import type { Appearance } from "../lib/appearance";
import { libraryPath, openLink, revealPath } from "../lib/ipc";
import { fileManagerName } from "../lib/platform";
import { Button, Keys } from "../ui/Button";
import { Dialog } from "../ui/Dialog";
import { Segmented } from "../ui/Segmented";
import { SHORTCUTS } from "./shortcuts";

export type SettingsTab = "general" | "assistant" | "shortcuts";

const MCP_GUIDE_URL = "https://github.com/harshnagouda/openarchi/blob/main/docs/mcp.md";
const CLAUDE_CODE_COMMAND = "claude mcp add openarchi -- /path/to/openarchi-mcp";
const CLAUDE_DESKTOP_CONFIG = `"openarchi": { "command": "/path/to/openarchi-mcp" }`;

type Props = {
  tab: SettingsTab;
  onTab: (tab: SettingsTab) => void;
  onClose: () => void;
  appearance: Appearance;
  onAppearance: (appearance: Appearance) => void;
  onTour: () => void;
};

export function Settings({ tab, onTab, onClose, appearance, onAppearance, onTour }: Props) {
  return (
    <Dialog title="Settings" width={480} onClose={onClose}>
      <div className="settings">
        <Segmented
          label="Settings section"
          value={tab}
          options={[
            { value: "general", label: "General" },
            { value: "assistant", label: "Assistant" },
            { value: "shortcuts", label: "Shortcuts" },
          ]}
          onChange={onTab}
        />
        {tab === "general" && <General appearance={appearance} onAppearance={onAppearance} onTour={onTour} />}
        {tab === "assistant" && <Assistant />}
        {tab === "shortcuts" && <Shortcuts />}
      </div>
    </Dialog>
  );
}

function General({ appearance, onAppearance, onTour }: Pick<Props, "appearance" | "onAppearance" | "onTour">) {
  const [path, setPath] = useState<string | null>(null);

  useEffect(() => {
    libraryPath()
      .then(setPath)
      .catch(() => setPath(null));
  }, []);

  return (
    <div className="settings-page">
      <div className="settings-row">
        <span className="settings-label">Appearance</span>
        <Segmented
          label="Appearance"
          value={appearance}
          options={[
            { value: "system", label: "System" },
            { value: "light", label: "Light" },
            { value: "dark", label: "Dark" },
            { value: "amoled", label: "AMOLED" },
          ]}
          onChange={onAppearance}
        />
      </div>
      <div className="settings-row settings-row-top">
        <span className="settings-label">Library</span>
        <div className="settings-value">
          <code className="path">{path ?? "Not available"}</code>
          {path && (
            <Button variant="secondary" icon="folder" onClick={() => revealPath(path)}>
              Show in {fileManagerName}
            </Button>
          )}
          <span className="field-note">Projects, backups and Recently Deleted live here. Assistants use the same folder.</span>
        </div>
      </div>
      <div className="settings-row">
        <span className="settings-label">Tour</span>
        <Button variant="secondary" onClick={onTour}>
          Show the tour again
        </Button>
      </div>
    </div>
  );
}

function CopyLine({ text }: { text: string }) {
  const [isCopied, setCopied] = useState(false);
  return (
    <div className="copy-line">
      <code>{text}</code>
      <Button
        icon={isCopied ? "check" : undefined}
        onClick={() => {
          navigator.clipboard.writeText(text).then(() => {
            setCopied(true);
            window.setTimeout(() => setCopied(false), 1500);
          });
        }}
      >
        {isCopied ? "Copied" : "Copy"}
      </Button>
    </div>
  );
}

function Assistant() {
  return (
    <div className="settings-page prose">
      <p>
        An assistant such as Claude can work in your projects through <code>openarchi-mcp</code>, a small program that
        ships with openarchi. Ask for a site analysis in plain words; the assistant draws the site, picks the diagram and
        palette, and exports. Its changes show up here as they happen, and every one can be undone.
      </p>
      <h4>Claude Code</h4>
      <CopyLine text={CLAUDE_CODE_COMMAND} />
      <h4>Claude Desktop</h4>
      <p>Settings › Developer › Edit Config, then add this under <code>mcpServers</code> and restart:</p>
      <CopyLine text={CLAUDE_DESKTOP_CONFIG} />
      <p className="field-note">Replace /path/to/openarchi-mcp with where the program is on your computer.</p>
      <Button variant="secondary" onClick={() => openLink(MCP_GUIDE_URL)}>
        Read the full guide
      </Button>
    </div>
  );
}

function Shortcuts() {
  return (
    <div className="settings-page">
      {SHORTCUTS.map((group) => (
        <div key={group.title} className="shortcut-group">
          <h4>{group.title}</h4>
          {group.items.map((item) => (
            <div key={item.label} className="shortcut-row">
              <span>{item.label}</span>
              <span className="shortcut-keys">
                {item.keys.map((key) => (
                  <Keys key={key} shortcut={key} />
                ))}
              </span>
            </div>
          ))}
        </div>
      ))}
    </div>
  );
}
