import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
  type KeyboardEvent,
  type ReactNode,
} from "react";
import { createPortal } from "react-dom";

export type AipSelectOption = {
  value: string;
  label: ReactNode;
  disabled?: boolean;
};

type SelectPosition = {
  top: number;
  left: number;
  width: number;
  maxHeight: number;
  placement: "above" | "below";
  ready: boolean;
};

type AipSelectProps = {
  id: string;
  label: ReactNode;
  value: string;
  options: readonly AipSelectOption[];
  onChange: (value: string) => void;
  description?: ReactNode;
  disabled?: boolean;
};

const VIEWPORT_PADDING = 8;
const MENU_GAP = 4;
const MIN_MENU_HEIGHT = 96;

function stableIdPart(value: string): string {
  return value.replace(/[^a-z0-9_-]+/gi, "-").replace(/^-|-$/g, "") || "value";
}

function optionId(selectId: string, value: string): string {
  return `${selectId}-option-${stableIdPart(value)}`;
}

function firstEnabledIndex(options: readonly AipSelectOption[]): number {
  return options.findIndex((option) => !option.disabled);
}

function lastEnabledIndex(options: readonly AipSelectOption[]): number {
  for (let index = options.length - 1; index >= 0; index -= 1) {
    if (!options[index]?.disabled) return index;
  }
  return -1;
}

function selectedIndex(
  options: readonly AipSelectOption[],
  value: string,
): number {
  const selected = options.findIndex((option) => option.value === value);
  return selected >= 0 && !options[selected]?.disabled
    ? selected
    : firstEnabledIndex(options);
}

function nextEnabledIndex(
  options: readonly AipSelectOption[],
  current: number,
  direction: 1 | -1,
): number {
  if (options.length === 0) return -1;
  let index = current;
  for (let step = 0; step < options.length; step += 1) {
    index = (index + direction + options.length) % options.length;
    if (!options[index]?.disabled) return index;
  }
  return current;
}

export function AipSelect({
  id,
  label,
  value,
  options,
  onChange,
  description,
  disabled = false,
}: AipSelectProps) {
  const selectId = id;
  const labelId = `${selectId}-label`;
  const descriptionId = `${selectId}-description`;
  const listboxId = `${selectId}-listbox`;
  const triggerRef = useRef<HTMLButtonElement>(null);
  const rootRef = useRef<HTMLLabelElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);
  const [open, setOpen] = useState(false);
  const [activeIndex, setActiveIndex] = useState(() =>
    selectedIndex(options, value),
  );
  const [position, setPosition] = useState<SelectPosition>({
    top: 0,
    left: 0,
    width: 0,
    maxHeight: MIN_MENU_HEIGHT,
    placement: "below",
    ready: false,
  });

  const updatePosition = useCallback(() => {
    const trigger = triggerRef.current;
    const menu = menuRef.current;
    if (trigger === null || menu === null) return;
    const rect = trigger.getBoundingClientRect();
    const menuHeight =
      menu.getBoundingClientRect().height ||
      menu.offsetHeight ||
      Math.min(options.length * 38, 320);
    const spaceBelow = Math.max(
      0,
      window.innerHeight - rect.bottom - VIEWPORT_PADDING,
    );
    const spaceAbove = Math.max(0, rect.top - VIEWPORT_PADDING);
    const placement =
      menuHeight > spaceBelow && spaceAbove > spaceBelow ? "above" : "below";
    const availableHeight = placement === "above" ? spaceAbove : spaceBelow;
    const maxHeight = Math.max(MIN_MENU_HEIGHT, availableHeight);
    const left = Math.min(
      Math.max(VIEWPORT_PADDING, rect.left),
      Math.max(
        VIEWPORT_PADDING,
        window.innerWidth - rect.width - VIEWPORT_PADDING,
      ),
    );
    const top =
      placement === "above"
        ? Math.max(
            VIEWPORT_PADDING,
            rect.top - Math.min(menuHeight, maxHeight) - MENU_GAP,
          )
        : rect.bottom + MENU_GAP;
    setPosition({
      top,
      left,
      width: rect.width,
      maxHeight,
      placement,
      ready: true,
    });
  }, [options.length]);

  useEffect(() => {
    if (!open) return;
    const onPointerDown = (event: PointerEvent) => {
      const target = event.target;
      if (!(target instanceof Node)) return;
      if (
        rootRef.current?.contains(target) ||
        menuRef.current?.contains(target)
      )
        return;
      setOpen(false);
    };
    document.addEventListener("pointerdown", onPointerDown);
    return () => document.removeEventListener("pointerdown", onPointerDown);
  }, [open]);

  useLayoutEffect(() => {
    if (!open) return;
    updatePosition();
    const onViewportChange = () => updatePosition();
    window.addEventListener("resize", onViewportChange);
    window.addEventListener("scroll", onViewportChange, true);
    return () => {
      window.removeEventListener("resize", onViewportChange);
      window.removeEventListener("scroll", onViewportChange, true);
    };
  }, [open, updatePosition]);

  useEffect(() => {
    if (!open) setPosition((current) => ({ ...current, ready: false }));
  }, [open]);

  const close = useCallback(() => {
    setOpen(false);
    triggerRef.current?.focus();
  }, []);

  const choose = useCallback(
    (index: number) => {
      const option = options[index];
      if (option === undefined || option.disabled) return;
      onChange(option.value);
      close();
    },
    [close, onChange, options],
  );

  function moveActive(direction: 1 | -1) {
    setActiveIndex((current) => nextEnabledIndex(options, current, direction));
  }

  function handleKeyDown(event: KeyboardEvent<HTMLButtonElement>) {
    const key = event.key;
    if (!open) {
      if (!["ArrowDown", "ArrowUp", "Home", "End", "Enter", " "].includes(key))
        return;
      event.preventDefault();
      setActiveIndex(selectedIndex(options, value));
      setOpen(true);
      return;
    }
    if (key === "ArrowDown" || key === "ArrowUp") {
      event.preventDefault();
      moveActive(key === "ArrowDown" ? 1 : -1);
    } else if (key === "Home" || key === "End") {
      event.preventDefault();
      setActiveIndex(
        key === "Home" ? firstEnabledIndex(options) : lastEnabledIndex(options),
      );
    } else if (key === "Enter" || key === " ") {
      event.preventDefault();
      choose(activeIndex);
    } else if (key === "Escape") {
      event.preventDefault();
      close();
    }
  }

  const activeOptionId =
    activeIndex >= 0 && options[activeIndex] !== undefined
      ? optionId(selectId, options[activeIndex]!.value)
      : undefined;
  const selected = options.find((option) => option.value === value);
  const selectedLabel = selected?.label ?? value;

  const menu = open
    ? createPortal(
        <div
          id={listboxId}
          ref={menuRef}
          className="aip-select-menu"
          role="listbox"
          aria-labelledby={labelId}
          data-placement={position.placement}
          style={{
            top: position.top,
            left: position.left,
            width: position.width,
            maxHeight: position.maxHeight,
            visibility: position.ready ? "visible" : "hidden",
          }}
        >
          {options.map((option, index) => (
            <div
              key={option.value}
              id={optionId(selectId, option.value)}
              className="aip-select-option"
              role="option"
              aria-selected={option.value === value}
              aria-disabled={option.disabled || undefined}
              data-active={index === activeIndex || undefined}
              data-value={option.value}
              onMouseDown={(event) => event.preventDefault()}
              onMouseEnter={() => {
                if (!option.disabled) setActiveIndex(index);
              }}
              onClick={() => choose(index)}
            >
              {option.label}
            </div>
          ))}
        </div>,
        document.body,
      )
    : null;

  return (
    <label ref={rootRef} className="aip-select" data-aip-select={selectId}>
      <span id={labelId} className="aip-select-label">
        {label}
      </span>
      <button
        id={`${selectId}-trigger`}
        ref={triggerRef}
        type="button"
        className="aip-select-trigger"
        aria-haspopup="listbox"
        aria-controls={listboxId}
        aria-owns={open ? listboxId : undefined}
        aria-expanded={open}
        aria-labelledby={labelId}
        aria-describedby={description ? descriptionId : undefined}
        aria-activedescendant={open ? activeOptionId : undefined}
        data-value={value}
        disabled={disabled}
        onClick={() => {
          if (open) {
            close();
          } else {
            setActiveIndex(selectedIndex(options, value));
            setOpen(true);
          }
        }}
        onKeyDown={handleKeyDown}
      >
        <span>{selectedLabel}</span>
        <span className="aip-select-chevron" aria-hidden="true">
          ▾
        </span>
      </button>
      {description ? (
        <span id={descriptionId} className="aip-select-description">
          {description}
        </span>
      ) : null}
      {menu}
    </label>
  );
}
