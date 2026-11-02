// "Go to element…" (Ctrl+P) and "Go to relationship…" (Ctrl+Shift+O), flows §6.1: the keyboard
// path to any element or relationship of the view, including those outside the viewport (AC-2,
// AC-3). A Base UI dialog with an inline autocomplete, the command palette pattern of Base UI.
import { Autocomplete } from "@base-ui/react/autocomplete";
import { Dialog } from "@base-ui/react/dialog";

export interface GoToItem {
  value: string;
  label: string;
}

export function GoToDialog({
  title,
  items,
  open,
  onOpenChange,
  onChoose,
}: {
  title: string;
  items: readonly GoToItem[];
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onChoose: (value: string) => void;
}) {
  return (
    <Dialog.Root open={open} onOpenChange={onOpenChange}>
      <Dialog.Portal>
        <Dialog.Backdrop className="fixed inset-0 bg-black/20" />
        <Dialog.Popup
          // The chosen element takes focus itself; returning it to the opener would undo that.
          finalFocus={false}
          className="fixed top-24 left-1/2 flex w-[min(32rem,calc(100vw-2rem))] -translate-x-1/2 flex-col rounded-lg border border-border-hairline bg-surface-panel shadow-xl"
        >
          <Dialog.Title className="sr-only">{title}</Dialog.Title>
          <Autocomplete.Root open inline items={items} autoHighlight="always" keepHighlight>
            <Autocomplete.Input
              aria-label={title}
              placeholder={`${title}…`}
              className="h-10 border-b border-border-hairline bg-transparent px-3 outline-none placeholder:text-text-muted"
            />
            <Autocomplete.Empty className="px-3 py-4 text-text-muted empty:hidden">
              Nothing matches.
            </Autocomplete.Empty>
            <Autocomplete.List className="max-h-80 overflow-auto py-1">
              {(item: GoToItem) => (
                <Autocomplete.Item
                  key={item.value}
                  value={item}
                  onClick={() => onChoose(item.value)}
                  className="flex min-h-8 cursor-default items-center px-3 data-highlighted:bg-surface-hover"
                >
                  {item.label}
                </Autocomplete.Item>
              )}
            </Autocomplete.List>
          </Autocomplete.Root>
        </Dialog.Popup>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
