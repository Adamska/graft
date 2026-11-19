import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { type MenuState, useUiStore } from "../../store/useUiStore";

const EDGE_MARGIN = 8;

/** The single context menu of the window; any component opens it through the UI store. */
export function ContextMenu() {
  const menu = useUiStore((state) => state.menu);
  return menu ? <Menu key={`${menu.x}:${menu.y}`} menu={menu} /> : null;
}

function Menu({ menu }: { menu: MenuState }) {
  const close = useUiStore((state) => state.closeMenu);
  const list = useRef<HTMLUListElement>(null);
  const [position, setPosition] = useState({ left: menu.x, top: menu.y });
  const [cursor, setCursor] = useState(-1);
  const enabled = menu.items.map((item, index) => (item.disabled ? -1 : index)).filter((index) => index >= 0);

  /* Flip the menu back inside the window when it would overflow the right or bottom edge. */
  useLayoutEffect(() => {
    const bounds = list.current?.getBoundingClientRect();
    if (!bounds) return;
    setPosition({
      left: Math.max(EDGE_MARGIN, Math.min(menu.x, window.innerWidth - bounds.width - EDGE_MARGIN)),
      top: Math.max(EDGE_MARGIN, Math.min(menu.y, window.innerHeight - bounds.height - EDGE_MARGIN)),
    });
    list.current?.focus();
  }, [menu]);

  useEffect(() => {
    window.addEventListener("blur", close);
    window.addEventListener("resize", close);
    return () => {
      window.removeEventListener("blur", close);
      window.removeEventListener("resize", close);
    };
  }, [close]);

  const run = (index: number) => {
    const item = menu.items[index];
    if (!item || item.disabled) return;
    close();
    void item.run();
  };

  const onKeyDown = (event: React.KeyboardEvent) => {
    const at = enabled.indexOf(cursor);
    if (event.key === "ArrowDown") setCursor(enabled[(at + 1) % enabled.length] ?? -1);
    else if (event.key === "ArrowUp") setCursor(enabled[(at - 1 + enabled.length) % enabled.length] ?? -1);
    else if (event.key === "Enter" || event.key === " ") run(cursor);
    else if (event.key === "Escape") close();
    event.preventDefault();
    event.stopPropagation();
  };

  return (
    <div className="fixed inset-0 z-50" onPointerDown={close} onContextMenu={(event) => event.preventDefault()} onWheel={close}>
      <ul
        ref={list}
        role="menu"
        tabIndex={-1}
        style={position}
        onPointerDown={(event) => event.stopPropagation()}
        onKeyDown={onKeyDown}
        className="absolute min-w-[220px] max-w-[380px] rounded-panel border border-line-strong bg-raised py-1 text-[12.5px] shadow-float outline-none"
      >
        {menu.items.map((item, index) => (
          <li key={item.label} role="none" className={item.separated ? "mt-1 border-t border-line pt-1" : ""}>
            <button
              type="button"
              role="menuitem"
              disabled={item.disabled}
              onPointerMove={() => setCursor(index)}
              onClick={() => run(index)}
              className={`flex h-7 w-full items-center gap-6 px-3 text-left disabled:text-faint ${item.danger ? "text-del" : "text-ink"} ${
                index === cursor && !item.disabled ? "bg-active" : ""
              }`}
            >
              <span className="min-w-0 flex-1 truncate">{item.label}</span>
              {item.hint && <span className="shrink-0 font-mono text-[10.5px] text-slate">{item.hint}</span>}
            </button>
          </li>
        ))}
      </ul>
    </div>
  );
}
