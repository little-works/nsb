import { __render } from '@/shared/helpter';
import { Icon } from '@/components/icon';

export interface MenuItem {
  label: string;
  onSelect?: () => void;
  icon?: any;
  danger?: boolean;
  divider?: boolean;
  disabled?: boolean;
}

export interface MenuProps {
  open: boolean;
  items: MenuItem[];
  x?: number;
  y?: number;
  onClose?: () => void;
}

const props = withDefaults(defineProps<MenuProps>(), {
  x: 0,
  y: 0,
  onClose: () => {},
});

defineOptions({ name: 'Menu' });

export default __render<MenuProps>(() => {
  if (!props.open) return null;
  return (
    <div
      class="fixed z-dropdown min-w-40 rounded border border-outline-variant bg-surface-container-lowest py-1 shadow-xl"
      style={{ left: `${props.x}px`, top: `${props.y}px` }}
      onClick={(event) => event.stopPropagation()}
    >
      {props.items.map((item) => {
        const ItemIcon = item.icon;
        return (
          <div key={`${item.label}-${item.divider ? 'divider' : 'item'}`}>
            {item.divider ? <div class="my-1 h-px bg-outline-variant" /> : null}
            <button
              class={[
                'flex h-9 w-full items-center gap-2 px-4 text-left text-sm',
                item.danger
                  ? 'text-error hover:bg-error-container'
                  : 'text-on-surface hover:bg-surface-container',
                item.disabled ? 'cursor-not-allowed opacity-50' : '',
              ]}
              type="button"
              disabled={item.disabled}
              onClick={() => {
                item.onSelect?.();
                props.onClose();
              }}
            >
              {ItemIcon ? (
                <Icon class="text-base">
                  <ItemIcon />
                </Icon>
              ) : null}
              {item.label}
            </button>
          </div>
        );
      })}
    </div>
  );
});
