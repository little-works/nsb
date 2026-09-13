import { __render } from '@/shared/helper';
import { TransitionGroup } from 'vue';
import Toast from './index.setup';
import { dismissToast, pauseToast, resumeToast, toasts } from './toast-manager';

defineOptions({ name: 'ToastHost' });

export default __render(() => (
  <TransitionGroup
    // @ts-expect-error
    class="fixed right-6 top-6 z-toast flex flex-col gap-3"
    name="toast"
    tag="div"
  >
    {toasts.value.map(({ id, ...toast }) => (
      <Toast
        key={id}
        {...toast}
        onClose={() => dismissToast(id)}
        onMouseenter={() => pauseToast(id)}
        onMouseleave={() => resumeToast(id)}
      />
    ))}
  </TransitionGroup>
));
