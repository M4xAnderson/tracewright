// The bottom pane (flows §2.2): one tab strip over the open tab's content. Each tab's content is
// its own region, "Bottom pane: <tab>", so the landmark names the open tab (flows §2.4).
import { Tabs } from "@base-ui/react/tabs";
import { useState } from "react";
import type { View } from "../../ipc/bindings/view";
import { BehaviorPane } from "../behavior/behavior-pane";
import { ProblemsPane } from "../problems/problems-pane";

type BottomTab = "behavior" | "problems";

export function BottomPane({ view }: { view: View }) {
  const [tab, setTab] = useState<BottomTab>("behavior");
  const tabClass =
    "h-8 cursor-pointer border-b-2 border-transparent px-2.5 text-text-secondary data-active:border-text-primary data-active:font-semibold data-active:text-text-primary";
  return (
    <Tabs.Root
      value={tab}
      onValueChange={(value: BottomTab) => setTab(value)}
      className="flex shrink-0 flex-col border-t border-border-hairline bg-surface-panel"
    >
      <Tabs.List
        aria-label="Bottom pane"
        className="flex gap-0.5 border-b border-border-hairline px-2"
      >
        <Tabs.Tab value="behavior" className={tabClass}>
          Behavior model
        </Tabs.Tab>
        <Tabs.Tab value="problems" className={tabClass}>
          Problems ({view.problems.length})
        </Tabs.Tab>
      </Tabs.List>
      <Tabs.Panel value="behavior">
        <BehaviorPane view={view} />
      </Tabs.Panel>
      <Tabs.Panel value="problems">
        <ProblemsPane view={view} />
      </Tabs.Panel>
    </Tabs.Root>
  );
}
