// The bar under the sheet: one tab per sheet, and on the right the tools that
// belong to the showing sheet: its pins, its highlights and its section line. A
// sheet of part of the site comes from a highlight's Open as sheet.

import type { ReactNode } from "react";
import type { Look, PinIcon } from "../../lib/types";
import { Button } from "../../ui/Button";
import { PinPalette } from "../pins/PinPalette";
import { TabBar, type Tab } from "./TabBar";
import { sheetName, type Workbook } from "./workbook";

type Props = {
  book: Workbook;
  index: number;
  look: Look;
  isReady: boolean;
  onSelect: (index: number) => void;
  onAdd: () => void;
  onRename: (index: number, name: string) => void;
  onDuplicate: (index: number) => void;
  onDelete: (index: number) => void;
  onMove: (from: number, to: number) => void;
  onLook: (next: Look) => void;
  onDropPin: (icon: PinIcon, clientX: number, clientY: number) => void;
  onPlacePin: (icon: PinIcon) => void;
  isDrawingSection: boolean;
  onSectionTool: () => void;
  /** The highlight tool, which SheetStep wires to its own state. */
  highlightTools: ReactNode;
};

export function SheetTabs(props: Props) {
  const { book, index, look, isReady } = props;
  const tabs: Tab[] = book.sheets.map((sheet, at) => ({ key: `sheet-${at}`, label: sheetName(sheet, at) }));
  const arePinsHidden = look.hide_pins ?? false;
  const isSectionView = look.view.mode === "section";
  const sectionTip = isSectionView ? "Section lines are drawn on the plan" : look.section ? "Draw the section line again" : "Draw a section line";

  return (
    <TabBar
      tabs={tabs}
      activeIndex={index}
      onSelect={props.onSelect}
      onAdd={props.onAdd}
      onRename={props.onRename}
      onDuplicate={props.onDuplicate}
      onDelete={props.onDelete}
      onMove={props.onMove}
    >
      <PinPalette
        isDisabled={!isReady || arePinsHidden}
        disabledTip={arePinsHidden ? "Pins are hidden on this sheet" : "Pins need a fetched site"}
        onDrop={props.onDropPin}
        onPlace={props.onPlacePin}
      />
      <Button
        icon={arePinsHidden ? "eyeOff" : "eye"}
        tip={arePinsHidden ? "Show pins on this sheet" : "Hide pins on this sheet"}
        tipSide="above"
        disabled={!isReady}
        onClick={() => props.onLook({ ...look, hide_pins: !arePinsHidden })}
      />
      <span className="topbar-divider" />
      {props.highlightTools}
      <span className="topbar-divider" />
      <Button
        icon="section"
        className={look.section ? "section-button is-set" : "section-button"}
        tip={sectionTip}
        tipSide="above"
        shortcut="s"
        isActive={props.isDrawingSection}
        disabled={!isReady || isSectionView}
        onClick={props.onSectionTool}
      />
    </TabBar>
  );
}
