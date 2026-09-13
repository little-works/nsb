import { __render } from '@/shared/helper';
import { Button } from '@/components/button';
import { nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import type { ProxyGroup } from './types';

export interface GroupDockContentProps {
  groups: ProxyGroup[];
  activeGroupTitle: string;
  onGroupSelect?: (groupTitle: string) => void;
}

const props = defineProps<GroupDockContentProps>();

const scrollContainer = ref<HTMLElement | null>(null);
const canScrollUp = ref(false);
const canScrollDown = ref(false);
let resizeObserver: ResizeObserver | undefined;

function updateScrollIndicators() {
  const element = scrollContainer.value;
  if (!element) {
    return;
  }

  canScrollUp.value = element.scrollTop > 1;
  canScrollDown.value =
    element.scrollHeight - element.clientHeight - element.scrollTop > 1;
}

function observeScrollContainer() {
  void nextTick(() => {
    if (!scrollContainer.value) {
      return;
    }

    resizeObserver = new ResizeObserver(updateScrollIndicators);
    resizeObserver.observe(scrollContainer.value);
    updateScrollIndicators();
  });
}

onMounted(observeScrollContainer);
onBeforeUnmount(() => resizeObserver?.disconnect());
watch(
  () => props.groups.map((group) => group.title).join('\u0000'),
  () => void nextTick(updateScrollIndicators),
);

defineOptions({ name: 'GroupDockContent' });

export default __render<GroupDockContentProps>(() => (
  <div class="relative flex min-h-0 flex-col">
    <div
      ref={scrollContainer}
      class="flex min-h-0 flex-1 flex-col gap-1 overflow-y-auto
        [-ms-overflow-style:none] [scrollbar-width:none]
        [&::-webkit-scrollbar]:hidden"
      onScroll={updateScrollIndicators}
    >
      {props.groups.map((group) => (
        <Button
          key={group.title}
          title={group.title}
          shape="square"
          size="sm"
          variant={props.activeGroupTitle === group.title ? 'solid' : 'ghost'}
          class="font-bold"
          onClick={() => props.onGroupSelect?.(group.title)}
        >
          {group.title.slice(0, 2).toUpperCase()}
        </Button>
      ))}
    </div>

    {canScrollUp.value ? (
      <div
        class="pointer-events-none absolute inset-x-0 top-0 h-5
        bg-gradient-to-b from-surface-container-lowest to-transparent"
      ></div>
    ) : null}
    {canScrollDown.value ? (
      <div
        class="pointer-events-none absolute inset-x-0 bottom-0 h-5
        bg-gradient-to-t from-surface-container-lowest to-transparent"
      ></div>
    ) : null}
  </div>
));
